pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod i18n;

use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tracks voice connection info for a guild (used for kick notification and auto-reconnection).
#[derive(Clone)]
pub struct VoiceConnectionInfo {
    /// The text channel where `/connect` was originally used (for notifications).
    pub text_channel_id: serenity::ChannelId,
    /// The voice channel the bot was connected to (for auto-reconnection).
    pub voice_channel_id: serenity::ChannelId,
}

/// Shared application state, injected into every command via Poise's Context.
pub struct Data {
    pub db_pool: SqlitePool,
    pub start_time: std::time::Instant,
    pub http_client: reqwest::Client,
    /// Maps guild_id → voice connection info (text channel + voice channel).
    /// Used for kick notification and auto-reconnection after network drops.
    /// Wrapped in Arc so background reconnection tasks can access it.
    pub voice_connections: Arc<RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>>,
}

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
