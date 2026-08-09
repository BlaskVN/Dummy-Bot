use anyhow::Result;
use poise::serenity_prelude::GuildId;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLogHealth {
    Disabled,
    Healthy,
    Degraded,
}

impl MessageLogHealth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            _ => Self::Disabled,
        }
    }
}

pub async fn reconcile(
    pool: &SqlitePool,
    guild_id: GuildId,
    message_content_enabled: bool,
) -> Result<(MessageLogHealth, bool)> {
    let row = sqlx::query_as::<_, (i64, String, i64)>(
        "SELECT enabled, health, degraded_warning_sent FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await?;
    let Some((enabled, old_health, warning_sent)) = row else {
        return Ok((MessageLogHealth::Disabled, false));
    };
    let health = if enabled == 0 {
        MessageLogHealth::Disabled
    } else if message_content_enabled {
        MessageLogHealth::Healthy
    } else {
        MessageLogHealth::Degraded
    };
    let warn = health == MessageLogHealth::Degraded
        && (old_health != health.as_str() || warning_sent == 0);
    let warning_sent = if health == MessageLogHealth::Degraded {
        warning_sent
    } else {
        0
    };
    sqlx::query(
        "UPDATE message_log_config SET health = ?, degraded_warning_sent = ? WHERE guild_id = ?",
    )
    .bind(health.as_str())
    .bind(warning_sent)
    .bind(guild_id.to_string())
    .execute(pool)
    .await?;
    Ok((health, warn))
}

pub async fn mark_warning_sent(pool: &SqlitePool, guild_id: GuildId) -> Result<()> {
    sqlx::query("UPDATE message_log_config SET degraded_warning_sent = 1 WHERE guild_id = ? AND health = 'degraded'")
        .bind(guild_id.to_string()).execute(pool).await?;
    Ok(())
}

pub async fn current_health(pool: &SqlitePool, guild_id: GuildId) -> Result<MessageLogHealth> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT health FROM message_log_config WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(pool)
            .await?
            .as_deref()
            .map(MessageLogHealth::parse)
            .unwrap_or(MessageLogHealth::Disabled),
    )
}

#[cfg(test)]
mod tests {
    use super::{MessageLogHealth, mark_warning_sent, reconcile};
    use crate::database::init_db;
    use poise::serenity_prelude::GuildId;

    #[tokio::test]
    async fn tracks_healthy_degraded_restart_and_recovery() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-message-health-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO message_log_config (guild_id, log_channel_id, enabled) VALUES ('1', '2', 1)").execute(&pool).await.unwrap();
        assert_eq!(
            reconcile(&pool, GuildId::new(1), true).await.unwrap(),
            (MessageLogHealth::Healthy, false)
        );
        assert_eq!(
            reconcile(&pool, GuildId::new(1), false).await.unwrap(),
            (MessageLogHealth::Degraded, true)
        );
        mark_warning_sent(&pool, GuildId::new(1)).await.unwrap();
        assert_eq!(
            reconcile(&pool, GuildId::new(1), false).await.unwrap(),
            (MessageLogHealth::Degraded, false)
        );
        assert_eq!(
            reconcile(&pool, GuildId::new(1), true).await.unwrap(),
            (MessageLogHealth::Healthy, false)
        );
        assert_eq!(
            reconcile(&pool, GuildId::new(1), false).await.unwrap(),
            (MessageLogHealth::Degraded, true)
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
