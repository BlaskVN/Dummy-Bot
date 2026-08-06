CREATE TABLE guild_timezone (
    guild_id TEXT PRIMARY KEY,
    iana_name TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
