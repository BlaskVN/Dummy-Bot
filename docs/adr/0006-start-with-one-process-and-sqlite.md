# Start with one process and SQLite

For the initially expected adoption of at most a few dozen Guilds, the bot runs as one process backed by SQLite even though installation is public. PostgreSQL, sharding, and distributed coordination are deferred until measured adoption or load demonstrates the need, accepting a future migration in exchange for simpler operation now.
