# Spec: Riot RSO VALORANT Player Data & Guild Leaderboard

## Problem Statement
Prior to this feature, the bot used unverified tracker.gg URLs saved as `Tracker Profile Link`. This had multiple architectural and policy compliance issues:
1. **Terms of Service & Cloudflare Blocking**: Tracker Network prohibits web scraping and blocks bot IPs via Cloudflare.
2. **Riot Developer Terms Compliance**: Riot Games requires player opt-in (via Riot Sign On - RSO) before player data can be displayed or ranked, preventing harassment and doxxing.
3. **Lack of In-Discord Experience**: Members could only click outbound links rather than seeing their rank, RR, and competitive standings directly inside Discord.

## Goals & Architecture
Implement official, policy-compliant player data retrieval and a Guild VALORANT Leaderboard based on ADR-0004 and CONTEXT.md:
- **Linked Riot Account**: Stored per Discord user (`user_id`, `puuid`, `game_name`, `tag_line`, `region`).
- **Guild Profile Visibility**: Per-Guild member consent (`guild_id`, `user_id`, `visible`). Privacy by default: linking alone leaves the profile hidden across all Guilds until explicitly enabled in that Guild.
- **Riot API Seam (`RiotApiClient`)**: Deep module with `HttpRiotApiClient` for official production calls and `MockRiotApiClient` for offline/deterministic testing. Provides `PlayerRankedData` (tier name, RR, act wins) with competitive scoring for leaderboards.
- **Slash Commands**:
  - `/valorant profile [member]`: Displays player info, rank, RR, and act wins. Enforces Guild Profile Visibility when viewing others.
  - `/valorant leaderboard`: Ranks all visible linked members in the current Guild by competitive rank and RR.
  - `/valorant visibility <enable|disable|status>`: Toggles or checks per-Guild consent.
  - `/valorant link <riot_id> [region]` & `/valorant unlink`: Manages account connection.
- **Elimination of Tracker Network Integration**:
  - Drop table `valorant_tracker_profile`.
  - Remove all tracker scraping, URL parsing, and command handlers.
  - Update `CONTEXT.md` and `ADR-0004` to reflect the removal of `Tracker Profile Link`.
- **Companion Website (Minimal Web Surface per CONTEXT.md & ADR-0004)**:
  - Minimal web surface for bot info, legal policies (Privacy, Terms), Riot Sign On login/redirect, account unlinking, and data-deletion requests.
  - Redirect URI compatible with `https://blaskvn.github.io/Dummy-Bot/auth/callback`.
- **Trilingual Localization**:
  - All user-facing strings and command parameters shipped in English (`en`), Vietnamese (`vi`), and Japanese (`ja`).

## Success Criteria
1. Tracker table, commands, and URL validator completely removed.
2. Linked accounts and per-Guild visibility isolated and tested in SQLite.
3. Guild data cleanup in `delete_guild_data` includes `valorant_guild_visibility`.
4. Mock Riot API client generates deterministic rank data across all 25 tiers from Iron 1 to Radiant.
5. Production Riot RSO configurations loaded and documented (`RIOT_RSO_CLIENT_ID`, `RIOT_RSO_CLIENT_SECRET`, `RIOT_RSO_REDIRECT_URI`).
6. Minimal Companion Website hosted under `docs/` providing RSO login, callback handling, privacy policy, terms, and data-deletion requests.
7. All 104+ unit tests and integration tests pass cleanly with zero Clippy warnings.
