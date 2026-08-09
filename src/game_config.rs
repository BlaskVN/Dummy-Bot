use anyhow::{Result, bail};
use poise::serenity_prelude::{ChannelId, GuildId, RoleId};
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GameConfig {
    pub role_id: String,
    pub game_key: String,
    pub display_name: String,
    pub game_channel_id: String,
    pub primary_voice_channel_id: String,
    pub activity_application_id: Option<String>,
    pub activity_name: String,
}

impl GameConfig {
    pub fn role(&self) -> Result<RoleId> {
        Ok(RoleId::new(self.role_id.parse()?))
    }

    pub fn game_channel(&self) -> Result<ChannelId> {
        Ok(ChannelId::new(self.game_channel_id.parse()?))
    }

    pub fn primary_voice_channel(&self) -> Result<ChannelId> {
        Ok(ChannelId::new(self.primary_voice_channel_id.parse()?))
    }
}

pub struct NewGameConfig<'a> {
    pub role_id: RoleId,
    pub game_key: &'a str,
    pub display_name: &'a str,
    pub game_channel_id: ChannelId,
    pub primary_voice_channel_id: ChannelId,
    pub voice_pool: &'a [ChannelId],
    pub activity_application_id: Option<u64>,
    pub activity_name: &'a str,
}

pub async fn save_game_config(
    pool: &SqlitePool,
    guild_id: GuildId,
    config: NewGameConfig<'_>,
) -> Result<()> {
    if config.voice_pool.is_empty() || !config.voice_pool.contains(&config.primary_voice_channel_id)
    {
        bail!("Primary voice channel must be in the voice pool");
    }
    let mut transaction = pool.begin().await?;
    sqlx::query("INSERT INTO game_config (guild_id, role_id, game_key, display_name, game_channel_id, primary_voice_channel_id, activity_application_id, activity_name) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(guild_id) DO UPDATE SET role_id = excluded.role_id, game_key = excluded.game_key, display_name = excluded.display_name, game_channel_id = excluded.game_channel_id, primary_voice_channel_id = excluded.primary_voice_channel_id, activity_application_id = excluded.activity_application_id, activity_name = excluded.activity_name, updated_at = CURRENT_TIMESTAMP")
        .bind(guild_id.to_string()).bind(config.role_id.to_string()).bind(config.game_key)
        .bind(config.display_name).bind(config.game_channel_id.to_string())
        .bind(config.primary_voice_channel_id.to_string()).bind(config.activity_application_id.map(|id| id.to_string()))
        .bind(config.activity_name).execute(&mut *transaction).await?;
    sqlx::query("DELETE FROM game_voice_channel WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&mut *transaction)
        .await?;
    for channel in config.voice_pool {
        sqlx::query("INSERT INTO game_voice_channel (guild_id, channel_id) VALUES (?, ?)")
            .bind(guild_id.to_string())
            .bind(channel.to_string())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn game_config(
    pool: &SqlitePool,
    guild_id: GuildId,
) -> Result<Option<(GameConfig, Vec<ChannelId>)>> {
    let Some(config) = sqlx::query_as("SELECT role_id, game_key, display_name, game_channel_id, primary_voice_channel_id, activity_application_id, activity_name FROM game_config WHERE guild_id = ?")
        .bind(guild_id.to_string()).fetch_optional(pool).await?
    else {
        return Ok(None);
    };
    let channels: Vec<String> = sqlx::query_scalar(
        "SELECT channel_id FROM game_voice_channel WHERE guild_id = ? ORDER BY channel_id",
    )
    .bind(guild_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(Some((
        config,
        channels
            .into_iter()
            .map(|id| Ok(ChannelId::new(id.parse()?)))
            .collect::<Result<Vec<_>>>()?,
    )))
}

pub async fn clear_game_config(pool: &SqlitePool, guild_id: GuildId) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM game_voice_channel WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let deleted = sqlx::query("DELETE FROM game_config WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
        == 1;
    transaction.commit().await?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::{NewGameConfig, clear_game_config, game_config, save_game_config};
    use crate::database::init_db;
    use poise::serenity_prelude::{ChannelId, GuildId, RoleId};

    #[tokio::test]
    async fn round_trips_one_isolated_game_mapping() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-game-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let channels = [ChannelId::new(30), ChannelId::new(31)];
        save_game_config(
            &pool,
            GuildId::new(1),
            NewGameConfig {
                role_id: RoleId::new(10),
                game_key: "minecraft",
                display_name: "Minecraft",
                game_channel_id: ChannelId::new(20),
                primary_voice_channel_id: ChannelId::new(30),
                voice_pool: &channels,
                activity_application_id: Some(40),
                activity_name: "Minecraft",
            },
        )
        .await
        .unwrap();
        let (config, pool_channels) = game_config(&pool, GuildId::new(1)).await.unwrap().unwrap();
        assert_eq!(config.role().unwrap(), RoleId::new(10));
        assert_eq!(config.game_channel().unwrap(), ChannelId::new(20));
        assert_eq!(config.primary_voice_channel().unwrap(), ChannelId::new(30));
        assert_eq!(pool_channels, channels);
        assert!(game_config(&pool, GuildId::new(2)).await.unwrap().is_none());
        assert!(clear_game_config(&pool, GuildId::new(1)).await.unwrap());
        assert!(game_config(&pool, GuildId::new(1)).await.unwrap().is_none());
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
