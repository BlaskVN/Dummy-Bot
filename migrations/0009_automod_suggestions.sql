CREATE TABLE automod_suggestion (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'handled', 'rule_updated')),
    opened_at INTEGER NOT NULL,
    resolved_at INTEGER,
    resolver_user_id TEXT,
    delivery_status TEXT NOT NULL DEFAULT 'pending' CHECK (delivery_status IN ('pending', 'delivered', 'failed'))
);

CREATE UNIQUE INDEX automod_suggestion_one_open
ON automod_suggestion (guild_id, user_id, rule_id)
WHERE status = 'open';
