# Store minimal moderation evidence

Moderation Cases store identifiers, the action, reason, timestamp, status, and an optional Discord link to evidence, but never copy message content or attachments into the database. This sacrifices self-contained historical evidence to reduce sensitive-data retention and leaves message capture to the separately configured Message Log Channel.
