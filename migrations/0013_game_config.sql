CREATE TABLE game_config (
    guild_id TEXT PRIMARY KEY,
    role_id TEXT NOT NULL,
    game_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    game_channel_id TEXT NOT NULL,
    primary_voice_channel_id TEXT NOT NULL,
    activity_application_id TEXT,
    activity_name TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE game_voice_channel (
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    PRIMARY KEY (guild_id, channel_id),
    FOREIGN KEY (guild_id) REFERENCES game_config(guild_id) ON DELETE CASCADE
);
