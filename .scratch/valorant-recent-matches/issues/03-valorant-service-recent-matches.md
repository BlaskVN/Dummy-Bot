# 03: Implement ValorantService get_recent_matches with Privacy Enforcement

**What to build:**
Implement `get_recent_matches` in `src/valorant/service.rs`. When requesting matches for another member, verify that the target has enabled **Guild Profile Visibility** in the command Guild. Return domain error enums (`ValorantMatchesError`) on missing link or disabled visibility.

**Blocked by:** 02 (Extend RiotApiClient Seam with Recent Matches)

**Status:** complete

- [x] Define `ValorantMatchesError` enum with variants: `NotLinkedSelf`, `NotLinkedOther`, `HiddenOther`, `ApiForbidden`, `ApiError(String)`.
- [x] Implement `ValorantService::get_recent_matches(&self, requester_user_id: u64, target_user_id: u64, guild_id: GuildId, count: usize) -> Result<PlayerRecentMatchesData, ValorantMatchesError>`.
- [x] Add unit tests verifying:
  - Caller viewing self without linked account returns `NotLinkedSelf`.
  - Caller viewing another member without linked account returns `NotLinkedOther`.
  - Caller viewing another member who linked account but kept visibility false returns `HiddenOther`.
  - Caller viewing another member who enabled visibility returns matches.
  - Caller viewing self with linked account returns matches regardless of Guild visibility flag.
