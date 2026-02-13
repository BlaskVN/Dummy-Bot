pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod i18n;

use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Shared application state, injected into every command via Poise's Context.
pub struct Data {
    pub db_pool: SqlitePool,
    pub start_time: std::time::Instant,
    pub http_client: reqwest::Client,
    /// Maps guild_id → text channel where `/connect` was used.
    /// Used to notify when the bot is kicked from voice.
    pub voice_text_channels: RwLock<HashMap<serenity::GuildId, serenity::ChannelId>>,
}

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
