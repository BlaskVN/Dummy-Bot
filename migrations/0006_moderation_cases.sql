CREATE TABLE moderation_case_counter (
    guild_id TEXT PRIMARY KEY,
    last_number INTEGER NOT NULL CHECK (last_number > 0)
);

CREATE TABLE moderation_case (
    guild_id TEXT NOT NULL,
    case_number INTEGER NOT NULL CHECK (case_number > 0),
    action TEXT NOT NULL CHECK (action IN ('warn', 'kick', 'ban', 'timeout')),
    target_user_id TEXT NOT NULL,
    moderator_user_id TEXT NOT NULL,
    reason TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    evidence_url TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'voided')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    void_actor_user_id TEXT,
    void_reason TEXT,
    voided_at TIMESTAMP,
    PRIMARY KEY (guild_id, case_number),
    CHECK (
        (status = 'active' AND void_actor_user_id IS NULL AND void_reason IS NULL AND voided_at IS NULL)
        OR
        (status = 'voided' AND void_actor_user_id IS NOT NULL AND length(trim(void_reason)) > 0 AND voided_at IS NOT NULL)
    )
);
