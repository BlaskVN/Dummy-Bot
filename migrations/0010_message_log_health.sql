ALTER TABLE message_log_config ADD COLUMN health TEXT NOT NULL DEFAULT 'disabled'
CHECK (health IN ('disabled', 'healthy', 'degraded'));

ALTER TABLE message_log_config ADD COLUMN degraded_warning_sent INTEGER NOT NULL DEFAULT 0
CHECK (degraded_warning_sent IN (0, 1));
