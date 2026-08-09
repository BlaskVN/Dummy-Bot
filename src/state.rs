use crate::config::Config;
use crate::i18n::{Language, get_guild_language};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};

#[derive(Clone)]
pub struct VoiceConnectionInfo {
    pub text_channel_id: serenity::ChannelId,
    pub voice_channel_id: serenity::ChannelId,
}

pub struct Data {
    pub config: Arc<Config>,
    pub db_pool: SqlitePool,
    pub start_time: std::time::Instant,
    pub attachment_client: reqwest::Client,
    pub attachment_downloads: Arc<Semaphore>,
    pub voice_connections: Arc<RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>>,
    /// ponytail: one lock serializes rare session creation; shard by Guild if this becomes hot.
    pub game_session_creation: Arc<Mutex<()>>,
}

impl Data {
    pub fn default_language(&self) -> Language {
        Language::parse(&self.config.default_language)
    }

    pub async fn language(&self, guild_id: serenity::GuildId) -> Language {
        get_guild_language(&self.db_pool, guild_id, self.default_language()).await
    }
}
