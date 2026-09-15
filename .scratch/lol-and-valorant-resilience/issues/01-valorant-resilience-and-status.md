# 01: VALORANT API Key Resilience & Status Command

**What to build:**
1. Expose `get_platform_status(region: RiotRegion)` on `ValorantService`.
2. Add `/valorant status [region]` command in `src/commands/valorant.rs` utilizing `val-status-v1` (`/val/status/v1/platform-data`) to display real-time server health and maintenance incidents.
3. Refine HTTP 403 Forbidden error handling for `/valorant matches` and `/valorant profile`: ensure clear, localized messages explaining that match and personal rank endpoints require a Riot Production Key / RSO, preventing technical confusion.
4. Add trilingual localization (EN, VI, JA) in `src/i18n.rs`.

**Blocked by:** None (can start immediately).

**Status:** resolved

- [x] Add `get_platform_status` method to `ValorantService`.
- [x] Add `/valorant status [region]` subcommand to `src/commands/valorant.rs`.
- [x] Refine 403 Forbidden handling in `/valorant profile` and `/valorant matches`.
- [x] Add unit tests for `get_platform_status` and status formatting.
- [x] Verify test suite passes (`cargo test`).
