use anyhow::Result;
use poise::serenity_prelude::GuildId;
use sqlx::SqlitePool;

const SEVEN_DAYS_SECONDS: i64 = 7 * 24 * 60 * 60;

pub struct ExecutionMetadata {
    pub guild_id: GuildId,
    pub user_id: u64,
    pub rule_id: u64,
    pub action_type: u8,
    pub channel_id: Option<u64>,
    pub message_id: Option<u64>,
    pub alert_message_id: Option<u64>,
}

pub async fn record_execution(
    pool: &SqlitePool,
    execution: &ExecutionMetadata,
    observed_at: i64,
) -> Result<bool> {
    // ponytail: five-second fallback can merge identical ID-less events; use Discord delivery IDs if exposed later.
    let identity = execution
        .message_id
        .or(execution.alert_message_id)
        .map_or(observed_at / 5, |id| id as i64);
    let delivery_key = format!(
        "{}:{}:{}:{}:{}:{}",
        execution.guild_id,
        execution.user_id,
        execution.rule_id,
        execution.action_type,
        execution.channel_id.unwrap_or_default(),
        identity
    );
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM automod_execution WHERE observed_at < ?")
        .bind(observed_at - SEVEN_DAYS_SECONDS)
        .execute(&mut *transaction)
        .await?;
    let inserted = sqlx::query("INSERT OR IGNORE INTO automod_execution (delivery_key, guild_id, user_id, rule_id, action_type, channel_id, message_id, observed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(delivery_key).bind(execution.guild_id.to_string()).bind(execution.user_id.to_string())
        .bind(execution.rule_id.to_string()).bind(execution.action_type)
        .bind(execution.channel_id.map(|id| id.to_string())).bind(execution.message_id.map(|id| id.to_string()))
        .bind(observed_at).execute(&mut *transaction).await?.rows_affected() == 1;
    transaction.commit().await?;
    Ok(inserted)
}

pub async fn observer_enabled(pool: &SqlitePool, guild_id: GuildId) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT enabled FROM automod_observer_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await?
        == Some(1))
}

pub async fn set_observer_enabled(
    pool: &SqlitePool,
    guild_id: GuildId,
    enabled: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO automod_observer_config (guild_id, enabled) VALUES (?, ?)\n         ON CONFLICT(guild_id) DO UPDATE SET enabled = excluded.enabled, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(enabled)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ExecutionMetadata, observer_enabled, record_execution, set_observer_enabled};
    use crate::database::init_db;
    use poise::serenity_prelude::GuildId;

    #[tokio::test]
    async fn observer_configuration_is_guild_scoped() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-automod-config-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        set_observer_enabled(&pool, GuildId::new(1), true)
            .await
            .unwrap();
        assert!(observer_enabled(&pool, GuildId::new(1)).await.unwrap());
        assert!(!observer_enabled(&pool, GuildId::new(2)).await.unwrap());
        set_observer_enabled(&pool, GuildId::new(1), false)
            .await
            .unwrap();
        assert!(!observer_enabled(&pool, GuildId::new(1)).await.unwrap());
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn executions_deduplicate_and_prune_without_content() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-automod-execution-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let execution = ExecutionMetadata {
            guild_id: GuildId::new(1),
            user_id: 2,
            rule_id: 3,
            action_type: 1,
            channel_id: Some(4),
            message_id: Some(5),
            alert_message_id: None,
        };
        assert!(
            record_execution(&pool, &execution, 1_000_000)
                .await
                .unwrap()
        );
        assert!(
            !record_execution(&pool, &execution, 1_000_001)
                .await
                .unwrap()
        );
        let other = ExecutionMetadata {
            guild_id: GuildId::new(2),
            ..execution
        };
        assert!(record_execution(&pool, &other, 1_000_001).await.unwrap());
        let later = ExecutionMetadata {
            message_id: Some(6),
            ..other
        };
        assert!(
            record_execution(&pool, &later, 1_000_000 + 7 * 24 * 60 * 60 + 2)
                .await
                .unwrap()
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM automod_execution")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('automod_execution')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(!columns.iter().any(|name| matches!(
            name.as_str(),
            "content" | "matched_content" | "matched_keyword"
        )));
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
