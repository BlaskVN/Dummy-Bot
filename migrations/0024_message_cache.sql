CREATE TABLE IF NOT EXISTS cached_message (
    message_id TEXT NOT NULL PRIMARY KEY,
    channel_id TEXT NOT NULL,
    guild_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_avatar_url TEXT NOT NULL,
    is_bot INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    attachments_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_cached_message_created_at ON cached_message(created_at);
CREATE INDEX IF NOT EXISTS idx_cached_message_channel ON cached_message(channel_id, message_id);
