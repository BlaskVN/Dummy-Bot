ALTER TABLE community_activity ADD COLUMN finalized_at INTEGER;

CREATE TABLE activity_attendance_interval (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    scheduled_event_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL CHECK (ended_at > started_at),
    UNIQUE (guild_id, scheduled_event_id, user_id, started_at, ended_at),
    FOREIGN KEY (guild_id, scheduled_event_id)
        REFERENCES community_activity(guild_id, scheduled_event_id) ON DELETE CASCADE
);

CREATE TABLE activity_member_game_aggregate (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    game_key TEXT NOT NULL,
    play_minutes INTEGER NOT NULL DEFAULT 0 CHECK (play_minutes >= 0),
    session_credits INTEGER NOT NULL DEFAULT 0 CHECK (session_credits >= 0),
    PRIMARY KEY (guild_id, user_id, game_key)
);

CREATE TABLE activity_member_aggregate (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    play_minutes INTEGER NOT NULL DEFAULT 0 CHECK (play_minutes >= 0),
    session_credits INTEGER NOT NULL DEFAULT 0 CHECK (session_credits >= 0),
    PRIMARY KEY (guild_id, user_id)
);

CREATE TABLE activity_completion (
    guild_id TEXT NOT NULL,
    source_key TEXT NOT NULL,
    user_id TEXT NOT NULL,
    game_key TEXT NOT NULL,
    play_minutes INTEGER NOT NULL CHECK (play_minutes >= 0),
    session_credit INTEGER NOT NULL CHECK (session_credit IN (0, 1)),
    completed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, source_key, user_id)
);
