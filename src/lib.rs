pub mod app;
pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod i18n;
pub mod permissions;
pub mod state;

pub use app::run;
pub use state::{Data, VoiceConnectionInfo};

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
