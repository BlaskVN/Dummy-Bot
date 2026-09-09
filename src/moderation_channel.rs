use anyhow::{Context as _, Result};
use poise::serenity_prelude::{ChannelId, ChannelType, GuildId};
use sqlx::SqlitePool;

/// Checks whether a channel is valid for use as a Moderation Channel.
///
/// Must belong to the specified Guild and be a text channel.
#[must_use]
pub fn valid_moderation_channel(
    guild_id: GuildId,
    channel_guild_id: GuildId,
    kind: ChannelType,
) -> bool {
    guild_id == channel_guild_id && kind == ChannelType::Text
}

/// Retrieve the configured Moderation Channel for a Guild, if any.
pub async fn get_moderation_channel(
    pool: &SqlitePool,
    guild_id: GuildId,
) -> Result<Option<ChannelId>> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await?;

    match result {
        Some(raw) => {
            let id: u64 = raw.parse().with_context(|| {
                format!("Invalid stored moderation channel ID '{raw}' for guild {guild_id}")
            })?;
            Ok(Some(ChannelId::new(id)))
        }
        None => Ok(None),
    }
}

/// Set or update the configured Moderation Channel for a Guild.
pub async fn set_moderation_channel(
    pool: &SqlitePool,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES (?, ?)
         ON CONFLICT(guild_id) DO UPDATE SET channel_id = excluded.channel_id, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Clear the configured Moderation Channel for a Guild.
///
/// Returns `true` if a channel configuration was removed, or `false` if none was set.
pub async fn clear_moderation_channel(pool: &SqlitePool, guild_id: GuildId) -> Result<bool> {
    let result = sqlx::query("DELETE FROM moderation_channel_config WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Check whether a Moderation Channel is configured for a Guild.
pub async fn is_moderation_channel_configured(
    pool: &SqlitePool,
    guild_id: GuildId,
) -> Result<bool> {
    let configured: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM moderation_channel_config WHERE guild_id = ?)",
    )
    .bind(guild_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use poise::serenity_prelude::{ChannelType, GuildId};

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy_bot_test_mod_chan_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = crate::database::init_db(&url, &directory).await.unwrap();
        (pool, directory)
    }

    #[test]
    fn valid_channel_checks() {
        let guild1 = GuildId::new(100);
        let guild2 = GuildId::new(200);

        assert!(valid_moderation_channel(guild1, guild1, ChannelType::Text));
        assert!(!valid_moderation_channel(guild1, guild2, ChannelType::Text));
        assert!(!valid_moderation_channel(
            guild1,
            guild1,
            ChannelType::Voice
        ));
        assert!(!valid_moderation_channel(guild1, guild1, ChannelType::News));
        assert!(!valid_moderation_channel(
            guild1,
            guild1,
            ChannelType::Category
        ));
    }

    #[tokio::test]
    async fn roundtrip_set_and_get() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(1);
        let channel_id = ChannelId::new(42);

        assert_eq!(get_moderation_channel(&pool, guild_id).await.unwrap(), None);

        set_moderation_channel(&pool, guild_id, channel_id)
            .await
            .unwrap();
        assert_eq!(
            get_moderation_channel(&pool, guild_id).await.unwrap(),
            Some(channel_id)
        );

        // Update to a new channel
        let new_channel_id = ChannelId::new(99);
        set_moderation_channel(&pool, guild_id, new_channel_id)
            .await
            .unwrap();
        assert_eq!(
            get_moderation_channel(&pool, guild_id).await.unwrap(),
            Some(new_channel_id)
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn clear_channel_removes_configuration() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(2);
        let channel_id = ChannelId::new(100);

        // Clearing when not configured returns false
        let cleared = clear_moderation_channel(&pool, guild_id).await.unwrap();
        assert!(!cleared);

        set_moderation_channel(&pool, guild_id, channel_id)
            .await
            .unwrap();
        assert_eq!(
            get_moderation_channel(&pool, guild_id).await.unwrap(),
            Some(channel_id)
        );

        // Clearing when configured returns true
        let cleared = clear_moderation_channel(&pool, guild_id).await.unwrap();
        assert!(cleared);

        assert_eq!(get_moderation_channel(&pool, guild_id).await.unwrap(), None);

        // Clearing again returns false
        let cleared_again = clear_moderation_channel(&pool, guild_id).await.unwrap();
        assert!(!cleared_again);

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn is_configured_reflects_state() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(3);
        let channel_id = ChannelId::new(200);

        assert!(
            !is_moderation_channel_configured(&pool, guild_id)
                .await
                .unwrap()
        );

        set_moderation_channel(&pool, guild_id, channel_id)
            .await
            .unwrap();
        assert!(
            is_moderation_channel_configured(&pool, guild_id)
                .await
                .unwrap()
        );

        clear_moderation_channel(&pool, guild_id).await.unwrap();
        assert!(
            !is_moderation_channel_configured(&pool, guild_id)
                .await
                .unwrap()
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn guild_isolation() {
        let (pool, dir) = setup_test_pool().await;
        let guild_a = GuildId::new(10);
        let guild_b = GuildId::new(20);
        let channel_a = ChannelId::new(1000);

        set_moderation_channel(&pool, guild_a, channel_a)
            .await
            .unwrap();

        assert_eq!(
            get_moderation_channel(&pool, guild_a).await.unwrap(),
            Some(channel_a)
        );
        assert_eq!(get_moderation_channel(&pool, guild_b).await.unwrap(), None);
        assert!(
            is_moderation_channel_configured(&pool, guild_a)
                .await
                .unwrap()
        );
        assert!(
            !is_moderation_channel_configured(&pool, guild_b)
                .await
                .unwrap()
        );

        clear_moderation_channel(&pool, guild_b).await.unwrap();
        assert_eq!(
            get_moderation_channel(&pool, guild_a).await.unwrap(),
            Some(channel_a)
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn get_moderation_channel_invalid_id() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(30);

        sqlx::query(
            "INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES ('30', 'invalid_id')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = get_moderation_channel(&pool, guild_id).await;
        assert!(result.is_err());

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
