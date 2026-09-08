# Integrate Commands, Handlers, and App with MessageLogService

Status: resolved
Blocked by: 04

## Overview
Refactor `src/handlers/message_log.rs`, `src/commands/configuration/logging.rs`, and `src/commands/configuration/settings.rs` to delegate to `MessageLogService`.

## Details
- Replace raw SQL queries in `logging.rs` and `settings.rs` with `message_log` service calls.
- Slim down `src/handlers/message_log.rs` into a thin adapter passing Serenity contexts into `MessageLogService`.
- Verify full test suite passes.

## Comments
- Replaced raw SQL queries in `logging.rs` and `settings.rs` with `message_log` methods.
- Reduced `src/handlers/message_log.rs` from ~973 lines to ~140 lines.
- All integration and unit tests pass without regression.

