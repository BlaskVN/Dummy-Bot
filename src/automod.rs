use anyhow::Result;
use poise::serenity_prelude::GuildId;
use sqlx::SqlitePool;

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
    use super::{observer_enabled, set_observer_enabled};
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
}
