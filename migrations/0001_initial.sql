CREATE TABLE IF NOT EXISTS guild_config (
    guild_id TEXT PRIMARY KEY,
    prefix TEXT NOT NULL,
    log_channel_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS message_log_config (
    guild_id TEXT PRIMARY KEY,
    log_channel_id TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS guild_language (
    guild_id TEXT PRIMARY KEY,
    language TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS bot_presence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    status TEXT NOT NULL,
    activity_kind TEXT,
    activity_text TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
