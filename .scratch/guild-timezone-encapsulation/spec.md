# Spec: Encapsulate Guild Time Zone Querying & Boundary Calculations

## Problem Statement
`src/timezone.rs` is currently a shallow pure-function calculation:
`parse(name: &str) -> Option<Tz>`
`next_five_am(now: DateTime<Utc>, timezone: Tz) -> Option<DateTime<Utc>>`
Callers across five separate modules duplicate identical raw SQL queries against `guild_timezone`:
- `src/commands/configuration/timezone.rs` (set, show, clear)
- `src/commands/configuration/game.rs`
- `src/commands/configuration/settings.rs`
- `src/handlers/game_session.rs`
- `src/word_puzzle_store.rs`

## Goals & Architecture
Deepen `src/timezone.rs` into a cohesive `GuildTimeZone` module:
- `get_timezone(pool, guild_id) -> Result<Option<Tz>>`: resolves guild time zone and parses into `chrono_tz::Tz`.
- `get_timezone_name(pool, guild_id) -> Result<Option<String>>`: returns configured IANA name string.
- `set_timezone(pool, guild_id, iana_name) -> Result<()>`: validates IANA name invariant upfront and saves.
- `clear_timezone(pool, guild_id) -> Result<()>`: resets guild time zone.
- `next_session_expiry(pool, guild_id, now) -> Result<Option<DateTime<Utc>>>`: combines time zone query, IANA resolution, and 05:00 local boundary calculation in one atomic call.
- Keep pure calculations `parse` and `next_five_am`.
- Replace all raw SQL queries across call sites with calls to `crate::timezone`.

## Success Criteria
1. Five duplicate raw SQL queries eliminated.
2. IANA validation enforced before database writes.
3. Unit and integration tests verify persistence, resolution, and boundary math.
