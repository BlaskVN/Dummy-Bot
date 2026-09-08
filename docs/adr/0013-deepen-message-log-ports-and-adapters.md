# Deepen Message Log Ingestion & Differential Engine with Ports & Adapters

Message logging historically scattered raw SQL queries across configuration commands (`/messagelog`, `/settings`) and concentrated attachment networking, SQLite TTL pruning, embed paging, and degraded health tracking into a monolithic 973-line event handler (`handlers/message_log.rs`).

We encapsulate all message logging lifecycle, configuration, health reconciliation, markdown diffing, and attachment archiving behind a deep `MessageLogService`. External interactions with Discord channels and HTTP CDN attachments are factored into explicit port traits (`MessageLogOutbox` and `AttachmentFetcher`).

This provides:
1. Encapsulation of database queries and health transitions, eliminating raw SQL in slash command handlers.
2. Complete testability of message deletion, bulk pruning, diff formatting, and degradation alerts using in-memory port doubles without live network I/O.
3. High locality: message caching, differential text generation, and quota management remain private implementation details within `src/message_log`.
