use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// Initialize the SQLite database connection pool.
///
/// Creates the `data/` directory if it doesn't exist and establishes
/// a connection pool with reasonable defaults for a Discord bot workload.
pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // Ensure the data directory exists for SQLite file
    if database_url.contains("data/") {
        tokio::fs::create_dir_all("data")
            .await
            .context("Failed to create data directory")?;
    }

    let pool = SqlitePool::connect(database_url)
        .await
        .context("Failed to connect to SQLite database")?;

    // Run migrations or create initial tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS guild_config (
            guild_id TEXT PRIMARY KEY,
            prefix TEXT NOT NULL DEFAULT '!',
            log_channel_id TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
        .execute(&pool)
        .await
        .context("Failed to create guild_config table")?;

    // Message logging configuration table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS message_log_config (
            guild_id TEXT PRIMARY KEY,
            log_channel_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
        .execute(&pool)
        .await
        .context("Failed to create message_log_config table")?;

    // Guild language preferences table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS guild_language (
            guild_id TEXT PRIMARY KEY,
            language TEXT NOT NULL DEFAULT 'en',
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
        .execute(&pool)
        .await
        .context("Failed to create guild_language table")?;

    tracing::info!("Database initialized successfully");
    Ok(pool)
}
