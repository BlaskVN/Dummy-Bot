CREATE UNIQUE INDEX one_open_game_activity
    ON community_activity(guild_id, game_key)
    WHERE kind = 'game' AND state IN ('scheduled', 'active');
