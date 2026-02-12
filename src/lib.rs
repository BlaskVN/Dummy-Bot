pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;

use sqlx::SqlitePool;

/// Shared application state, injected into every command via Poise's Context.
pub struct Data {
    pub db_pool: SqlitePool,
    pub start_time: std::time::Instant,
    pub http_client: reqwest::Client,
}

/// Centralized error type for the entire bot.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Poise context alias used across all command modules.
pub type Context<'a> = poise::Context<'a, Data, Error>;
