CREATE TABLE automod_execution (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delivery_key TEXT NOT NULL UNIQUE,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    action_type INTEGER NOT NULL,
    channel_id TEXT,
    message_id TEXT,
    observed_at INTEGER NOT NULL
);

CREATE INDEX automod_execution_threshold
ON automod_execution (guild_id, user_id, rule_id, observed_at);
