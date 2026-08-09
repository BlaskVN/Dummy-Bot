CREATE TABLE activity_opt_out (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, user_id)
);

CREATE TABLE activity_attendance (
    guild_id TEXT NOT NULL,
    scheduled_event_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    accumulated_seconds INTEGER NOT NULL DEFAULT 0 CHECK (accumulated_seconds >= 0),
    active_started_at INTEGER,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, scheduled_event_id, user_id),
    FOREIGN KEY (guild_id, scheduled_event_id)
        REFERENCES community_activity(guild_id, scheduled_event_id) ON DELETE CASCADE
);

CREATE INDEX activity_attendance_active
    ON activity_attendance(guild_id, scheduled_event_id, active_started_at);
