# Implement MessageLogService Ingestion & Differential Engine

Status: resolved
Blocked by: 01, 02, 03

## Overview
Implement the core `MessageLogService` coordinating message persistence, event processing (delete, update, bulk delete), attachment archiving, and outbox dispatch.

## Details
- Implement `MessageLogService` methods:
  - `save_message(pool, message)`
  - `handle_message_delete(outbox, fetcher, cache, pool, config, lang, channel_id, deleted_message_id, guild_id)`
  - `handle_message_update(outbox, cache, pool, config, lang, old_message, event)`
  - `handle_message_delete_bulk(outbox, cache, pool, config, lang, channel_id, deleted_ids, guild_id)`
  - `archive_purge_attachments(outbox, fetcher, pool, config, guild_id, messages)`
- Add in-memory tests exercising deletion and edit diff flows through `InMemoryOutbox`.

## Comments
- Implemented `MessageLogService` coordinating message cache persistence, deletion, edit diffs, bulk deletion, attachment archiving, and skipped attachment alerts.
- Added comprehensive in-memory tests verifying full pipeline with `InMemoryOutbox` and `MockAttachmentFetcher`.

