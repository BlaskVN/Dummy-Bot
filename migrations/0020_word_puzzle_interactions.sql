ALTER TABLE word_puzzle_session ADD COLUMN result_channel_id TEXT;
ALTER TABLE word_puzzle_session ADD COLUMN summary_claimed_at INTEGER;

CREATE TABLE word_puzzle_interaction (
    delivery_key TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL,
    session_id INTEGER NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('playing', 'won', 'lost')),
    created_at INTEGER NOT NULL
);
