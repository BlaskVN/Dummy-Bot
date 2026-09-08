use anyhow::Result;
use poise::serenity_prelude::{ChannelId, GuildId};
use sqlx::SqlitePool;

use super::models::{MessageLogConfig, MessageLogHealth};

/// Fetch the current message logging configuration for a guild.
pub async fn get_config(pool: &SqlitePool, guild_id: GuildId) -> Result<Option<MessageLogConfig>> {
    let row = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT log_channel_id, enabled, health FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some((channel_id_str, enabled, health_str)) => {
            let channel_id: u64 = channel_id_str.parse()?;
            Ok(Some(MessageLogConfig {
                channel_id: ChannelId::new(channel_id),
                enabled: enabled == 1,
                health: MessageLogHealth::parse(&health_str),
            }))
        }
        None => Ok(None),
    }
}

/// Fetch the configured and enabled message log channel for a guild, if any.
pub async fn get_log_channel(pool: &SqlitePool, guild_id: GuildId) -> Result<Option<ChannelId>> {
    let config = get_config(pool, guild_id).await?;
    match config {
        Some(cfg) if cfg.enabled => Ok(Some(cfg.channel_id)),
        _ => Ok(None),
    }
}

/// Enable message logging for a guild, persisting channel configuration and reconciling initial health.
pub async fn enable(
    pool: &SqlitePool,
    guild_id: GuildId,
    log_channel_id: ChannelId,
    message_content_enabled: bool,
) -> Result<(MessageLogHealth, bool)> {
    sqlx::query(
        "INSERT INTO message_log_config (guild_id, log_channel_id, enabled)
         VALUES (?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET log_channel_id = excluded.log_channel_id, enabled = 1",
    )
    .bind(guild_id.to_string())
    .bind(log_channel_id.to_string())
    .execute(pool)
    .await?;

    reconcile(pool, guild_id, message_content_enabled).await
}

/// Enable message logging and automatically dispatch degraded warning through the outbox port.
pub async fn enable_with_outbox<O: super::ports::MessageLogOutbox>(
    pool: &SqlitePool,
    guild_id: GuildId,
    log_channel_id: ChannelId,
    message_content_enabled: bool,
    outbox: Option<(&O, poise::serenity_prelude::CreateMessage)>,
) -> Result<(MessageLogHealth, bool)> {
    let (health, warn) = enable(pool, guild_id, log_channel_id, message_content_enabled).await?;
    if warn
        && let Some((outbox_impl, warning_msg)) = outbox
        && outbox_impl
            .send_message(log_channel_id, warning_msg)
            .await
            .is_ok()
    {
        mark_warning_sent(pool, guild_id).await?;
    }
    Ok((health, warn))
}

/// Convenience helper to fetch a guild's message log status and localized description.
pub async fn status(
    pool: &SqlitePool,
    guild_id: GuildId,
    lang: crate::i18n::Language,
) -> Result<(MessageLogHealth, String)> {
    let health = current_health(pool, guild_id).await?;
    let config = get_config(pool, guild_id).await?;
    let desc = match config {
        Some(cfg) => format_status_description(lang, &cfg),
        None => format_status_description(
            lang,
            &MessageLogConfig {
                channel_id: ChannelId::new(0),
                enabled: false,
                health: MessageLogHealth::Disabled,
            },
        ),
    };
    Ok((health, desc))
}

pub fn format_status_description(lang: crate::i18n::Language, config: &MessageLogConfig) -> String {
    use crate::i18n::{TranslationKey, t, tf};
    let status = if config.enabled {
        t(lang, TranslationKey::MessageLogStatusEnabled)
    } else {
        t(lang, TranslationKey::MessageLogStatusDisabled)
    };

    let status_label = t(lang, TranslationKey::MessageLogStatus);
    let channel_text = tf(
        lang,
        TranslationKey::MessageLogChannel,
        &[&config.channel_id],
    );
    let health_key = match config.health {
        MessageLogHealth::Disabled => TranslationKey::MessageLogHealthDisabled,
        MessageLogHealth::Healthy => TranslationKey::MessageLogHealthHealthy,
        MessageLogHealth::Degraded => TranslationKey::MessageLogHealthDegraded,
    };
    let health = t(lang, health_key);
    let health_text = tf(lang, TranslationKey::MessageLogHealth, &[&health]);

    format!(
        "{} {}\n{}\n{}",
        status_label, status, channel_text, health_text
    )
}

/// Disable message logging for a guild. Returns true if configuration was present and disabled.
pub async fn disable(
    pool: &SqlitePool,
    guild_id: GuildId,
    message_content_enabled: bool,
) -> Result<bool> {
    let result = sqlx::query("UPDATE message_log_config SET enabled = 0 WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Ok(false);
    }

    reconcile(pool, guild_id, message_content_enabled).await?;
    Ok(true)
}

/// Reconcile health state for a single guild based on configuration and Message Content intent availability.
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

/// Mark that a degraded message log warning was delivered to the guild's log channel.
pub async fn mark_warning_sent(pool: &SqlitePool, guild_id: GuildId) -> Result<()> {
    sqlx::query("UPDATE message_log_config SET degraded_warning_sent = 1 WHERE guild_id = ? AND health = 'degraded'")
        .bind(guild_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Query the currently recorded health of a guild's message logging.
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

/// Retrieve all enabled guild configurations.
pub async fn load_enabled_guilds(pool: &SqlitePool) -> Result<Vec<(GuildId, ChannelId)>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT guild_id, log_channel_id FROM message_log_config WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for (guild, channel) in rows {
        if let (Ok(g), Ok(c)) = (guild.parse::<u64>(), channel.parse::<u64>()) {
            result.push((GuildId::new(g), ChannelId::new(c)));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;

    #[tokio::test]
    async fn tracks_healthy_degraded_restart_and_recovery() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-ml-health-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        enable(&pool, GuildId::new(1), ChannelId::new(2), true)
            .await
            .unwrap();

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

        let config = get_config(&pool, GuildId::new(1)).await.unwrap().unwrap();
        assert_eq!(config.channel_id, ChannelId::new(2));
        assert!(config.enabled);
        assert_eq!(config.health, MessageLogHealth::Healthy);

        let disabled = disable(&pool, GuildId::new(1), true).await.unwrap();
        assert!(disabled);
        let config_after = get_config(&pool, GuildId::new(1)).await.unwrap().unwrap();
        assert!(!config_after.enabled);
        assert_eq!(config_after.health, MessageLogHealth::Disabled);
    }
}
