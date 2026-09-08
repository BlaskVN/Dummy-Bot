DROP TABLE IF EXISTS valorant_tracker_profile;

CREATE TABLE valorant_linked_account (
    user_id TEXT PRIMARY KEY,
    puuid TEXT NOT NULL UNIQUE,
    game_name TEXT NOT NULL,
    tag_line TEXT NOT NULL,
    region TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE valorant_guild_visibility (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    visible INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, user_id)
);
