# Spec: Deepen Message Log Ingestion & Differential Engine

## Problem Statement
The message logging system currently exhibits several architectural shortcomings:
1. **Wide & Leaky Handler**: `src/handlers/message_log.rs` spans 973 lines, entangling raw Serenity event handling, SQLite queries, attachment HTTP fetching, embed limit chunking, and markdown escaping into a single file.
2. **Scattered Database Logic**: Raw SQL queries on `message_log_config` are duplicated in `src/commands/configuration/logging.rs`, `src/commands/configuration/settings.rs`, `src/message_log_health.rs`, and `src/handlers/message_log.rs`.
3. **Shallow Health Wrapper**: `src/message_log_health.rs` is a shallow wrapper that exposes low-level SQL and enum string transformations rather than a cohesive domain model.
4. **Poor Testability of I/O Pipeline**: Message logging logic cannot be tested end-to-end without real HTTP clients and Discord API connections, limiting test coverage to pure formatting helper functions.

## Goals & Architecture
We will refactor message logging into a deep module (`crate::message_log`):
- **Thin Public Interface**: Exposes cohesive domain operations (`enable`, `enable_with_outbox`, `disable`, `status`, `reconcile`, `get_config`, `get_log_channel`), domain service `MessageLogService` (providing `save_message`, `handle_message_delete`, `handle_message_update`, `handle_message_delete_bulk`, `archive_purge_attachments`, `reconcile_all_health`), domain models (`MessageLogConfig`, `MessageLogHealth`, `MessageLogOptions`, `DeletedMessageView`, `EditedMessageView`, `PurgedMessageSummary`), and ports.
- **Ports & Adapters (Hexagonal Architecture)**:
  - `MessageLogOutbox` port: Abstraction for delivering embeds and files to a Discord channel (`DiscordOutbox` for production, `InMemoryOutbox` for testing).
  - `AttachmentFetcher` port: Abstraction for downloading attachments with byte limits and CDN verification (`HttpAttachmentFetcher` for production, `MockAttachmentFetcher` for testing).
- **Differential & Formatting Engine**: Pure formatting sub-module managing markdown escaping, quote truncation, before/after diff layouts, jump links, and multi-embed pagination adhering to Discord limits.
- **Encapsulated Configuration & Health**: Centralized persistence for `message_log_config`, health status tracking (`Healthy`, `Degraded`, `Disabled`), and degraded warning dispatch.

## Success Criteria
1. All existing test suites pass (including `v1_1_isolation` and `v2_prefix_inventory`).
2. Raw SQL queries on `message_log_config` are removed from slash command handlers.
3. `src/handlers/message_log.rs` is reduced to a concise dispatcher coordinating events to `MessageLogService`.
4. The ports & adapters seam enables unit and integration tests for message delete, update, bulk delete, and degraded warning flows without network access.
