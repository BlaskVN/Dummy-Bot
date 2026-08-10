pub mod activity_aggregate;
pub mod activity_privacy;
pub mod app;
pub mod attendance;
pub mod automod;
pub mod commands;
pub mod community;
pub mod config;
pub mod database;
pub mod error;
pub mod game_config;
pub mod handlers;
pub mod i18n;
pub mod message_log_health;
pub mod moderation_cases;
pub mod permissions;
pub mod state;
pub mod timezone;

pub use app::run;
pub use state::{Data, VoiceConnectionInfo};

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
