CREATE TABLE word_puzzle_session (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL UNIQUE,
    creator_id TEXT NOT NULL,
    answer TEXT NOT NULL CHECK (length(answer) = 5),
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    deadline_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE word_puzzle_participant (
    session_id INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'joined'
        CHECK (status IN ('joined', 'playing', 'won', 'lost')),
    PRIMARY KEY (session_id, user_id),
    FOREIGN KEY (session_id) REFERENCES word_puzzle_session(id) ON DELETE CASCADE
);

CREATE TABLE word_puzzle_guess (
    session_id INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt BETWEEN 1 AND 6),
    word TEXT NOT NULL CHECK (length(word) = 5),
    marks TEXT NOT NULL CHECK (length(marks) = 5),
    PRIMARY KEY (session_id, user_id, attempt),
    FOREIGN KEY (session_id, user_id)
        REFERENCES word_puzzle_participant(session_id, user_id) ON DELETE CASCADE
);

CREATE TABLE word_puzzle_completion (
    session_id INTEGER NOT NULL,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    completed_at INTEGER NOT NULL,
    PRIMARY KEY (session_id, user_id)
);
