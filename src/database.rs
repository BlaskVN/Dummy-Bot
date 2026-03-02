use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// Persistent presence configuration stored across bot restarts.
pub struct BotPresenceRecord {
    pub status: String,
    pub activity_kind: Option<String>,
    pub activity_text: Option<String>,
}

/// Upsert the bot's persistent presence into the database.
/// Only call this when duration is permanent (0 or unset).
pub async fn save_bot_presence(
    pool: &SqlitePool,
    status: &str,
    activity_kind: Option<&str>,
    activity_text: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO bot_presence (id, status, activity_kind, activity_text, updated_at)
         VALUES (1, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
             status        = excluded.status,
             activity_kind = excluded.activity_kind,
             activity_text = excluded.activity_text,
             updated_at    = CURRENT_TIMESTAMP",
    )
    .bind(status)
    .bind(activity_kind)
    .bind(activity_text)
    .execute(pool)
    .await
    .context("Failed to save bot presence")?;
    Ok(())
}

/// Load the persistent presence row (there is at most one row with id = 1).
pub async fn load_bot_presence(pool: &SqlitePool) -> Result<Option<BotPresenceRecord>> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT status, activity_kind, activity_text FROM bot_presence WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .context("Failed to load bot presence")?;

    Ok(row.map(|(status, activity_kind, activity_text)| BotPresenceRecord {
        status,
        activity_kind,
        activity_text,
    }))
}

/// Remove the persistent presence row so the bot starts with Discord's default.
pub async fn clear_bot_presence(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM bot_presence WHERE id = 1")
        .execute(pool)
        .await
        .context("Failed to clear bot presence")?;
    Ok(())
}

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

    // Persistent bot presence (single-row, id always = 1)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS bot_presence (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            status TEXT NOT NULL DEFAULT 'online',
            activity_kind TEXT,
            activity_text TEXT,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await
    .context("Failed to create bot_presence table")?;

    tracing::info!("Database initialized successfully");
    Ok(pool)
}
