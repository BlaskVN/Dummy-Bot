use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;

/// Persistent presence configuration stored across bot restarts.
pub struct BotPresenceRecord {
    pub status: String,
    pub activity_kind: Option<String>,
    pub activity_text: Option<String>,
}

pub struct DonationConfig {
    pub message: Option<String>,
    pub url: Option<String>,
    pub qr_filename: Option<String>,
}

pub async fn load_donation_config(pool: &SqlitePool) -> Result<Option<DonationConfig>> {
    Ok(
        sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
            "SELECT message, url, qr_filename FROM donation_config WHERE id = 1",
        )
        .fetch_optional(pool)
        .await?
        .map(|(message, url, qr_filename)| DonationConfig {
            message,
            url,
            qr_filename,
        }),
    )
}

pub async fn save_donation_config(
    pool: &SqlitePool,
    message: Option<&str>,
    url: Option<&str>,
    qr_filename: Option<&str>,
) -> Result<Option<String>> {
    let old = sqlx::query_scalar("SELECT qr_filename FROM donation_config WHERE id = 1")
        .fetch_optional(pool)
        .await?
        .flatten();
    sqlx::query(
        "INSERT INTO donation_config (id, message, url, qr_filename) VALUES (1, ?, ?, ?)\n         ON CONFLICT(id) DO UPDATE SET message = excluded.message, url = excluded.url, qr_filename = excluded.qr_filename, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(message)
    .bind(url)
    .bind(qr_filename)
    .execute(pool)
    .await?;
    Ok(old)
}

pub async fn clear_donation_config(pool: &SqlitePool) -> Result<Option<String>> {
    let old = sqlx::query_scalar("SELECT qr_filename FROM donation_config WHERE id = 1")
        .fetch_optional(pool)
        .await?
        .flatten();
    sqlx::query("DELETE FROM donation_config WHERE id = 1")
        .execute(pool)
        .await?;
    Ok(old)
}

/// Upsert the bot's persistent presence into the database.
/// Only call this when duration is permanent (0 or unset).
pub async fn save_bot_presence(
    pool: &SqlitePool,
    status: &str,
    activity_kind: Option<&str>,
    activity_text: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO bot_presence (id, status, activity_kind, activity_text, updated_at)
         VALUES (1, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
             status        = excluded.status,
             activity_kind = excluded.activity_kind,
             activity_text = excluded.activity_text,
             updated_at    = CURRENT_TIMESTAMP",
    )
    .bind(status)
    .bind(activity_kind)
    .bind(activity_text)
    .execute(pool)
    .await
    .context("Failed to save bot presence")?;
    Ok(())
}

/// Load the persistent presence row (there is at most one row with id = 1).
pub async fn load_bot_presence(pool: &SqlitePool) -> Result<Option<BotPresenceRecord>> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT status, activity_kind, activity_text FROM bot_presence WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .context("Failed to load bot presence")?;

    Ok(
        row.map(|(status, activity_kind, activity_text)| BotPresenceRecord {
            status,
            activity_kind,
            activity_text,
        }),
    )
}

/// Remove the persistent presence row so the bot starts with Discord's default.
pub async fn clear_bot_presence(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM bot_presence WHERE id = 1")
        .execute(pool)
        .await
        .context("Failed to clear bot presence")?;
    Ok(())
}

pub async fn guild_prefix(
    pool: &SqlitePool,
    guild_id: Option<poise::serenity_prelude::GuildId>,
    default: &str,
) -> Result<String> {
    let Some(guild_id) = guild_id else {
        return Ok(default.to_owned());
    };

    Ok(
        sqlx::query_scalar("SELECT prefix FROM guild_config WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(pool)
            .await
            .context("Failed to load guild prefix")?
            .unwrap_or_else(|| default.to_owned()),
    )
}

pub async fn message_log_channel(
    pool: &SqlitePool,
    guild_id: poise::serenity_prelude::GuildId,
) -> Result<Option<poise::serenity_prelude::ChannelId>> {
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await
    .context("Failed to load message log configuration")?;

    match row {
        Some((channel, 1)) => Ok(Some(poise::serenity_prelude::ChannelId::new(
            channel.parse().context("Invalid stored log channel ID")?,
        ))),
        _ => Ok(None),
    }
}

pub async fn delete_guild_data(
    pool: &SqlitePool,
    guild_id: poise::serenity_prelude::GuildId,
) -> Result<()> {
    let guild_id = guild_id.to_string();
    let mut transaction = pool.begin().await?;
    for table in [
        "activity_reward_grant",
        "activity_reward_config",
        "activity_completion",
        "activity_member_aggregate",
        "activity_member_game_aggregate",
        "activity_attendance_interval",
        "activity_attendance",
        "activity_opt_out",
        "game_voice_channel",
        "game_config",
        "community_activity_member",
        "community_activity",
        "automod_suggestion",
        "automod_execution",
        "automod_observer_config",
        "moderation_case",
        "moderation_case_counter",
        "moderation_channel_config",
        "guild_onboarding",
        "guild_timezone",
        "guild_language",
        "message_log_config",
        "guild_config",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE guild_id = ?"
        )))
        .bind(&guild_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

/// Claim a new Guild's one-time onboarding before attempting delivery.
pub async fn claim_guild_onboarding(
    pool: &SqlitePool,
    guild_id: poise::serenity_prelude::GuildId,
) -> Result<bool> {
    Ok(sqlx::query(
        "INSERT INTO guild_onboarding (guild_id) VALUES (?) ON CONFLICT(guild_id) DO NOTHING",
    )
    .bind(guild_id.to_string())
    .execute(pool)
    .await
    .context("Failed to claim guild onboarding")?
    .rows_affected()
        == 1)
}

/// Initialize the SQLite database connection pool.
///
/// Creates the `data/` directory if it doesn't exist and establishes
/// a connection pool with reasonable defaults for a Discord bot workload.
pub async fn init_db(database_url: &str, data_directory: &Path) -> Result<SqlitePool> {
    tokio::fs::create_dir_all(data_directory)
        .await
        .context("Failed to create data directory")?;

    let pool = SqlitePool::connect(database_url)
        .await
        .context("Failed to connect to SQLite database")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    tracing::info!("Database initialized successfully");
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::{claim_guild_onboarding, delete_guild_data, init_db};

    #[tokio::test]
    async fn applies_initial_migration() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-db-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = init_db(&url, &directory).await.unwrap();

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'guild_config'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(tables, 1);

        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn guild_timezones_are_isolated() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-timezone-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = init_db(&url, &directory).await.unwrap();
        sqlx::query("INSERT INTO guild_timezone (guild_id, iana_name) VALUES ('1', 'Asia/Bangkok'), ('2', 'America/New_York')")
            .execute(&pool).await.unwrap();
        let first: String =
            sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = '1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(first, "Asia/Bangkok");
        let second: String =
            sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = '2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(second, "America/New_York");
        sqlx::query("UPDATE guild_timezone SET iana_name = NULL WHERE guild_id = '1'")
            .execute(&pool)
            .await
            .unwrap();
        let cleared: Option<String> =
            sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = '1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(cleared.is_none());
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn moderation_channels_are_isolated() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-moderation-channel-test-{}",
            std::process::id()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = init_db(&url, &directory).await.unwrap();
        sqlx::query(
            "INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES ('1', '11'), ('2', '22')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let first: String = sqlx::query_scalar(
            "SELECT channel_id FROM moderation_channel_config WHERE guild_id = '1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let second: String = sqlx::query_scalar(
            "SELECT channel_id FROM moderation_channel_config WHERE guild_id = '2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((first.as_str(), second.as_str()), ("11", "22"));
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn onboarding_is_claimed_once_per_guild() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-onboarding-test-{}", std::process::id()));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = init_db(&url, &directory).await.unwrap();
        assert!(
            claim_guild_onboarding(&pool, poise::serenity_prelude::GuildId::new(1))
                .await
                .unwrap()
        );
        assert!(
            !claim_guild_onboarding(&pool, poise::serenity_prelude::GuildId::new(1))
                .await
                .unwrap()
        );
        assert!(
            claim_guild_onboarding(&pool, poise::serenity_prelude::GuildId::new(2))
                .await
                .unwrap()
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn guild_cleanup_is_complete_isolated_and_idempotent() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-cleanup-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        sqlx::raw_sql(
            "INSERT INTO guild_config (guild_id, prefix) VALUES ('1', '!'), ('2', '?');
             INSERT INTO message_log_config (guild_id, log_channel_id, enabled) VALUES ('1', '11', 1), ('2', '22', 1);
             INSERT INTO guild_language (guild_id, language) VALUES ('1', 'en'), ('2', 'vi');
             INSERT INTO guild_timezone (guild_id, iana_name) VALUES ('1', 'UTC'), ('2', 'Asia/Bangkok');
             INSERT INTO guild_onboarding (guild_id) VALUES ('1'), ('2');
             INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES ('1', '11'), ('2', '22');
             INSERT INTO moderation_case_counter (guild_id, last_number) VALUES ('1', 1), ('2', 1);
             INSERT INTO moderation_case (guild_id, case_number, action, target_user_id, moderator_user_id, reason) VALUES ('1', 1, 'warn', '3', '4', 'one'), ('2', 1, 'warn', '3', '4', 'two');
             INSERT INTO automod_observer_config (guild_id, enabled) VALUES ('1', 1), ('2', 1);
             INSERT INTO automod_execution (delivery_key, guild_id, user_id, rule_id, action_type, observed_at) VALUES ('one', '1', '3', '4', 1, 1), ('two', '2', '3', '4', 1, 1);
             INSERT INTO automod_suggestion (guild_id, user_id, rule_id, opened_at) VALUES ('1', '3', '4', 1), ('2', '3', '4', 1);
             INSERT INTO donation_config (id, message) VALUES (1, 'global');"
        ).execute(&pool).await.unwrap();
        let guild = poise::serenity_prelude::GuildId::new(1);
        delete_guild_data(&pool, guild).await.unwrap();
        delete_guild_data(&pool, guild).await.unwrap();
        let deleted_rows: i64 = sqlx::query_scalar(
            "SELECT (SELECT COUNT(*) FROM guild_config WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM message_log_config WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM guild_language WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM guild_timezone WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM guild_onboarding WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM moderation_channel_config WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM moderation_case_counter WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM moderation_case WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM automod_observer_config WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM automod_execution WHERE guild_id = '1') +
                    (SELECT COUNT(*) FROM automod_suggestion WHERE guild_id = '1')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(deleted_rows, 0);
        let other_guild_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM moderation_case WHERE guild_id = '2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(other_guild_rows, 1);
        let donation: String =
            sqlx::query_scalar("SELECT message FROM donation_config WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(donation, "global");
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
