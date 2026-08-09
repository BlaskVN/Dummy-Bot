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

pub async fn maybe_open_suggestion(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: u64,
    rule_id: u64,
    now: i64,
) -> Result<Option<i64>> {
    let mut transaction = pool.begin().await?;
    let open: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM automod_suggestion WHERE guild_id = ? AND user_id = ? AND rule_id = ? AND status = 'open'",
    )
    .bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string())
    .fetch_optional(&mut *transaction).await?;
    if open.is_some() {
        return Ok(None);
    }
    let resolved_at: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(resolved_at) FROM automod_suggestion WHERE guild_id = ? AND user_id = ? AND rule_id = ?",
    )
    .bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string())
    .fetch_one(&mut *transaction).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM automod_execution WHERE guild_id = ? AND user_id = ? AND rule_id = ? AND observed_at > ? AND observed_at >= ?",
    )
    .bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string())
    .bind(resolved_at.unwrap_or(i64::MIN)).bind(now - SEVEN_DAYS_SECONDS)
    .fetch_one(&mut *transaction).await?;
    if count < 3 {
        return Ok(None);
    }
    let id: Option<i64> = sqlx::query_scalar(
        "INSERT OR IGNORE INTO automod_suggestion (guild_id, user_id, rule_id, opened_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string()).bind(now)
    .fetch_one(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(id)
}

pub async fn handle_suggestion(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: u64,
    rule_id: u64,
    resolver: u64,
    now: i64,
) -> Result<bool> {
    Ok(sqlx::query("UPDATE automod_suggestion SET status = 'handled', resolved_at = ?, resolver_user_id = ? WHERE guild_id = ? AND user_id = ? AND rule_id = ? AND status = 'open'")
        .bind(now).bind(resolver.to_string()).bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string())
        .execute(pool).await?.rows_affected() == 1)
}

pub async fn resolve_rule_suggestions(
    pool: &SqlitePool,
    guild_id: GuildId,
    rule_id: u64,
    now: i64,
) -> Result<u64> {
    Ok(sqlx::query("UPDATE automod_suggestion SET status = 'rule_updated', resolved_at = ? WHERE guild_id = ? AND rule_id = ? AND status = 'open'")
        .bind(now).bind(guild_id.to_string()).bind(rule_id.to_string()).execute(pool).await?.rows_affected())
}

pub async fn open_suggestion_id(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: u64,
    rule_id: u64,
) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT id FROM automod_suggestion WHERE guild_id = ? AND user_id = ? AND rule_id = ? AND status = 'open'")
        .bind(guild_id.to_string()).bind(user_id.to_string()).bind(rule_id.to_string()).fetch_optional(pool).await?)
}

pub async fn mark_suggestion_delivery(pool: &SqlitePool, id: i64, delivered: bool) -> Result<()> {
    sqlx::query(
        "UPDATE automod_suggestion SET delivery_status = ? WHERE id = ? AND status = 'open'",
    )
    .bind(if delivered { "delivered" } else { "failed" })
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
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
    use super::{
        ExecutionMetadata, handle_suggestion, maybe_open_suggestion, observer_enabled,
        open_suggestion_id, record_execution, resolve_rule_suggestions, set_observer_enabled,
    };
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

    #[tokio::test]
    async fn suggestions_open_on_third_event_and_reset_after_resolution() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-automod-suggestion-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        for index in 1..=3 {
            let execution = ExecutionMetadata {
                guild_id: GuildId::new(1),
                user_id: 2,
                rule_id: 3,
                action_type: 1,
                channel_id: Some(4),
                message_id: Some(index),
                alert_message_id: None,
            };
            record_execution(&pool, &execution, 100 + index as i64)
                .await
                .unwrap();
            let opened = maybe_open_suggestion(&pool, GuildId::new(1), 2, 3, 100 + index as i64)
                .await
                .unwrap();
            assert_eq!(opened.is_some(), index == 3);
        }
        let fourth = ExecutionMetadata {
            guild_id: GuildId::new(1),
            user_id: 2,
            rule_id: 3,
            action_type: 1,
            channel_id: Some(4),
            message_id: Some(4),
            alert_message_id: None,
        };
        record_execution(&pool, &fourth, 104).await.unwrap();
        assert!(
            maybe_open_suggestion(&pool, GuildId::new(1), 2, 3, 104)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            resolve_rule_suggestions(&pool, GuildId::new(1), 99, 150)
                .await
                .unwrap(),
            0
        );
        assert!(
            open_suggestion_id(&pool, GuildId::new(1), 2, 3)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            handle_suggestion(&pool, GuildId::new(1), 2, 3, 9, 200)
                .await
                .unwrap()
        );
        assert!(
            !handle_suggestion(&pool, GuildId::new(1), 2, 3, 9, 200)
                .await
                .unwrap()
        );
        for index in 5..=7 {
            let execution = ExecutionMetadata {
                guild_id: GuildId::new(1),
                user_id: 2,
                rule_id: 3,
                action_type: 1,
                channel_id: Some(4),
                message_id: Some(index),
                alert_message_id: None,
            };
            record_execution(&pool, &execution, 200 + index as i64)
                .await
                .unwrap();
            let opened = maybe_open_suggestion(&pool, GuildId::new(1), 2, 3, 200 + index as i64)
                .await
                .unwrap();
            assert_eq!(opened.is_some(), index == 7);
        }
        assert_eq!(
            resolve_rule_suggestions(&pool, GuildId::new(1), 3, 300)
                .await
                .unwrap(),
            1
        );
        assert!(
            open_suggestion_id(&pool, GuildId::new(1), 2, 3)
                .await
                .unwrap()
                .is_none()
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
