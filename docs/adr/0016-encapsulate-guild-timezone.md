# Encapsulate Guild Time Zone Querying & Boundary Calculations

Guild time zone querying previously existed as a shallow pure-function calculation, forcing five separate modules across commands (`/timezone`, `/game`, `/settings`) and handlers (`game_session.rs`, `word_puzzle_store.rs`) to duplicate raw SQL queries against `guild_timezone`.

We deepened the `src/timezone.rs` module to encapsulate persistence (`set_timezone`, `clear_timezone`, `get_timezone_name`, `get_timezone`), IANA parsing, and 05:00 session expiry boundaries (`next_session_expiry`).

This provides:
1. High leverage: multiple call sites replace manual SQL querying and parsing with single domain function calls.
2. Invariant enforcement: IANA format is validated prior to database persistence.
3. High locality: boundary calculations, DST transitions, and SQL queries live together in a single deep module.
