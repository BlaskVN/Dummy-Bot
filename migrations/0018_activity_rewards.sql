CREATE TABLE activity_reward_config (
    guild_id TEXT PRIMARY KEY,
    role_id TEXT NOT NULL,
    level_threshold INTEGER NOT NULL CHECK (level_threshold > 0),
    ownership TEXT NOT NULL CHECK (ownership IN ('guild_owned', 'bot_owned')),
    health TEXT NOT NULL DEFAULT 'safe' CHECK (health IN ('safe', 'degraded')),
    degraded_reason TEXT,
    notification_sent INTEGER NOT NULL DEFAULT 0 CHECK (notification_sent IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE activity_reward_grant (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, user_id, role_id)
);
