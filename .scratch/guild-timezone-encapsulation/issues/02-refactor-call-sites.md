# Refactor Call Sites to Deep Timezone Module

Status: resolved

## Overview
Replace raw SQL queries in `commands/configuration/timezone.rs`, `commands/configuration/game.rs`, `commands/configuration/settings.rs`, and `handlers/game_session.rs` with `crate::timezone` calls.

## Details
- Refactor `commands/configuration/timezone.rs` to call `set_timezone`, `get_timezone_name`, `clear_timezone`.
- Refactor `commands/configuration/game.rs` to call `get_timezone`.
- Refactor `commands/configuration/settings.rs` to call `get_timezone_name`.
- Refactor `handlers/game_session.rs` to call `get_timezone` and `next_session_expiry`.
