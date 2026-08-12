pub mod activity_aggregate;
pub mod activity_privacy;
pub mod app;
pub mod attendance;
pub mod automod;
pub mod commands;
pub mod community;
pub mod config;
pub mod core;
pub mod database;
pub mod error;
pub mod game_config;
pub mod handlers;
pub mod i18n;
pub mod message_log_health;
pub mod moderation_cases;
pub mod permissions;
pub mod reward_roles;
pub mod state;
pub mod timezone;
pub mod ui;
pub mod word_puzzle;
pub mod word_puzzle_store;
pub mod word_set;

pub use app::run;
pub use state::{Data, VoiceConnectionInfo};

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
