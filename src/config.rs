use anyhow::{Context, Result};

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Discord bot token for authentication.
    pub discord_token: String,
    /// SQLite database connection URL.
    pub database_url: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required variables:
    /// - `DISCORD_TOKEN` — Bot token from Discord Developer Portal
    /// - `DATABASE_URL` — SQLite connection string (e.g. `sqlite:data/bot.db?mode=rwc`)
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .context("Missing DISCORD_TOKEN environment variable")?,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/bot.db?mode=rwc".to_string()),
        })
    }
}
