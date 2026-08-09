CREATE TABLE community_activity (
    guild_id TEXT NOT NULL,
    scheduled_event_id TEXT NOT NULL,
    host_user_id TEXT,
    kind TEXT NOT NULL CHECK (kind IN ('community', 'game')),
    game_key TEXT,
    capacity INTEGER CHECK (capacity IS NULL OR capacity > 0),
    state TEXT NOT NULL DEFAULT 'scheduled' CHECK (state IN ('scheduled', 'active', 'completed', 'canceled', 'deleted')),
    notification_sent INTEGER NOT NULL DEFAULT 0 CHECK (notification_sent IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, scheduled_event_id),
    CHECK ((kind = 'community' AND host_user_id IS NOT NULL) OR kind = 'game')
);
