# Centralize Message Log Configuration & Health State Machine

Status: resolved
Blocked by: 01

## Overview
Consolidate `message_log_config` queries and health state transitions into `src/message_log/health.rs` / `service.rs`.

## Details
- Provide centralized methods:
  - `get_config(pool, guild_id)`
  - `enable(pool, guild_id, channel_id, message_content_enabled, outbox)`
  - `disable(pool, guild_id, message_content_enabled)`
  - `reconcile(pool, guild_id, message_content_enabled)`
  - `reconcile_all(pool, outbox, message_content_enabled)`
  - `current_health(pool, guild_id)`
  - `mark_warning_sent(pool, guild_id)`
- Preserve backward compatibility for `crate::message_log_health` so existing tests (like `v1_1_isolation`) remain green.

## Comments
- Consolidated config queries and health transitions in `src/message_log/health.rs`.
- Preserved backward compatibility in `src/message_log_health.rs` re-exporting health types and functions.
- Verified healthy/degraded restart and recovery with unit tests.

