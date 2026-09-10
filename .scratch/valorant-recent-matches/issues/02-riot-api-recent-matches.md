# 02: Extend RiotApiClient Seam with Recent Matches

**What to build:**
Define `RecentMatchSummary` domain struct and add `get_recent_matches` to the `RiotApiClient` trait in `src/valorant/riot_api.rs`. Implement deterministic mock match generation in `MockRiotApiClient` and HTTP integration in `HttpRiotApiClient`.

**Blocked by:** None (can start immediately).

**Status:** complete

- [x] Add `RecentMatchSummary` struct to `src/valorant/riot_api.rs` with fields: `match_id`, `map_name`, `game_mode`, `game_start_millis`, `character`, `rounds_won`, `rounds_lost`, `kills`, `deaths`, `assists`, `won`.
- [x] Add `get_recent_matches` method to `RiotApiClient` trait returning `BoxFuture<'a, Result<Vec<RecentMatchSummary>>>`.
- [x] Implement `get_recent_matches` in `MockRiotApiClient` with realistic match history across agents, maps, and win/loss outcomes.
- [x] Implement `get_recent_matches` in `HttpRiotApiClient`.
- [x] Unit test `MockRiotApiClient::get_recent_matches` ensuring deterministic outcomes.
