# 02: League of Legends Domain Seam & API Client

**What to build:**
Implement the domain seam `LolApiClient` in `src/lol/api.rs` (or `client.rs`), including:
1. Routing regions & platform mappings (`LolPlatform`, `LolCluster`):
   - NA1, BR1, LA1, LA2, EUW1, EUN1, TR1, RU, KR, JP1, VN2, TW2, PH2, SG2, TH2, OC1.
   - Mapping from `RiotRegion` to default `LolPlatform`.
2. Domain DTOs:
   - `LolSummoner`: id, account_id, puuid, profile_icon_id, summoner_level.
   - `LolLeagueEntry`: queue_type (Solo/Duo, Flex), tier, division, lp, wins, losses.
   - `LolRecentMatch`: match_id, game_mode, game_start_millis, game_duration, champion_id, champion_name, kills, deaths, assists, win, items: [u32; 7], cs: u32.
   - `LolChampionMastery`: champion_id, champion_level, champion_points, last_play_time.
3. Seam trait `LolApiClient`:
   - `get_summoner_by_puuid(platform, puuid)`
   - `get_league_entries_by_puuid_or_summoner(platform, summoner_id, puuid)`
   - `get_recent_matches(platform_or_cluster, puuid, count)`
   - `get_top_champion_masteries(platform, puuid, count)`
   - `get_total_mastery_score(platform, puuid)`
4. `MockLolApiClient` for offline/deterministic TDD tests.
5. `HttpLolApiClient` connecting to official endpoints (`summoner-v4`, `league-v4`, `match-v5`, `champion-mastery-v4`).
6. Comprehensive unit tests for serialization, mock responses, and endpoint mapping.

**Blocked by:** None (can run in parallel or sequence).

**Status:** resolved

- [x] Define `LolPlatform`, `LolCluster`, and domain DTOs.
- [x] Implement `LolApiClient` trait.
- [x] Implement `MockLolApiClient` with deterministic mock data.
- [x] Implement `HttpLolApiClient` with reqwest and rate-limit / error handling.
- [x] Add unit tests verifying `MockLolApiClient` and payload deserialization.
