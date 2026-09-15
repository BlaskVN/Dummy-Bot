# 03: League of Legends Service & Consent Privacy Model

**What to build:**
Implement `LolService` in `src/lol/service.rs`:
1. Struct `LolService` holding `SqlitePool` and `Arc<dyn LolApiClient>`.
2. Reuse existing `valorant_linked_account` and `valorant_guild_visibility` storage models.
3. Privacy rules:
   - Requesting member viewing self (`requester == target`): always allowed if linked.
   - Requesting member viewing other (`requester != target`): requires target member to be linked AND to have `Guild Profile Visibility` enabled in the current Guild.
   - Return structured domain errors: `NotLinkedSelf`, `NotLinkedOther`, `HiddenOther`, `ApiForbidden`, `ApiNotFound`, `ApiError`, `Database`.
4. High-level methods:
   - `get_profile(guild_id, requester_id, target_id) -> Result<LolProfile, LolProfileError>`
   - `get_matches(guild_id, requester_id, target_id, count) -> Result<LolRecentMatchesData, LolMatchesError>`
   - `get_mastery(guild_id, requester_id, target_id, count) -> Result<LolMasteryData, LolMasteryError>`
5. TDD unit tests with in-memory SQLite and `MockLolApiClient` verifying:
   - Self profile/matches/mastery loading.
   - Other member hidden when visibility is false.
   - Other member permitted when visibility is true.
   - Unlinked member errors.

**Blocked by:** 02-lol-domain-api-client.md

**Status:** resolved

- [x] Implement `LolService` with `get_profile`, `get_matches`, `get_mastery`.
- [x] Implement domain errors for profile, matches, mastery.
- [x] Write exhaustive TDD unit tests with `MockLolApiClient` and SQLite test fixtures.
