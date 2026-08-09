ALTER TABLE community_activity ADD COLUMN expires_at INTEGER;

CREATE INDEX community_activity_expiry
    ON community_activity(expires_at)
    WHERE kind = 'game' AND state IN ('scheduled', 'active');
