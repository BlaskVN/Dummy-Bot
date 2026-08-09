CREATE TABLE community_activity_member (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    scheduled_event_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('participant', 'waitlisted')),
    joined_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    promoted_at TEXT,
    promotion_notification TEXT NOT NULL DEFAULT 'none'
        CHECK (promotion_notification IN ('none', 'pending', 'sending', 'delivered', 'failed')),
    UNIQUE (guild_id, scheduled_event_id, user_id),
    FOREIGN KEY (guild_id, scheduled_event_id)
        REFERENCES community_activity(guild_id, scheduled_event_id) ON DELETE CASCADE
);

CREATE INDEX community_activity_member_queue
    ON community_activity_member(guild_id, scheduled_event_id, state, sequence);
