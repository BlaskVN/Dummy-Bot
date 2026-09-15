# Spec: League of Legends (/lol) Module and VALORANT API Key Resilience

## Problem Statement

Dummy Bot currently supports VALORANT rank profiles, guild leaderboards, and recent matches. However, developers and users encounter two primary issues:
1. **VALORANT Development Key Limitations**:
   - When executing `/valorant matches` or `/valorant profile` on development or personal Riot API keys, Riot returns HTTP 403 Forbidden because match and personal rank endpoints are restricted to Production Keys / RSO. The bot currently surfaces generic or confusing error text that leaves users uncertain about what went wrong.
   - Users and server administrators have no way to check real-time VALORANT shard health or maintenance incidents, even though Riot provides `val-status-v1` (`/val/status/v1/platform-data`) which is completely open and functional on Development API keys.
2. **Missing League of Legends Support**:
   - League of Legends is Riot's flagship game and the most requested game integration. Currently, Dummy Bot lacks `/lol` commands despite Riot providing active Development API endpoints (`account-v1`, `summoner-v4`, `league-v4`, `match-v5`, `champion-mastery-v4`).
   - The bot needs `/lol profile`, `/lol matches`, and `/lol mastery` commands while upholding the existing privacy contract (requiring a global `Linked Riot Account` and per-guild `Guild Profile Visibility` consent before displaying data to other guild members).

## Solution

1. **VALORANT API Resilience & Status**:
   - Add `/valorant status [region]` command utilizing `val-status-v1` (`/val/status/v1/platform-data`) via `ValorantService::get_platform_status(region)`.
   - Display server name, operational status, active maintenance incidents, and platform incidents with localized status badges.
   - Refine HTTP 403 Forbidden error handling for `/valorant matches` and `/valorant profile` to clearly explain that VALORANT personal rank and match history endpoints require a Riot Production Key or RSO credentials under Riot Developer API policy.

2. **League of Legends (/lol) Module Implementation**:
   - **Domain Seam (`LolApiClient`)**:
     - Define `LolApiClient` port trait in `src/lol/api.rs`.
     - Implement `MockLolApiClient` for 100% offline, deterministic TDD test suite.
     - Implement `HttpLolApiClient` using `reqwest` with `X-Riot-Token` supporting regional clusters (`americas`, `asia`, `europe`, `sea`) and platform routing (`na1`, `euw1`, `vn2`, `kr`, `jp1`, etc.).
     - Integrate `account-v1`, `summoner-v4`, `league-v4`, `match-v5`, and `champion-mastery-v4`.
   - **Service Layer (`LolService`)**:
     - Implement `LolService` in `src/lol/service.rs`.
     - Reuse existing `Linked Riot Account` and `Guild Profile Visibility` in `src/valorant/storage.rs`.
     - Enforce consent: Self can always view their own profile/matches/mastery; inspecting another member requires that member to have enabled `Guild Profile Visibility` in the guild.
     - Provide atomic methods: `get_profile`, `get_matches`, `get_mastery`.
   - **Presentation Layer (`src/commands/lol.rs`)**:
     - Implement `/lol profile [member]`: Summoner level, icon, Solo/Duo & Flex rank (Tier, Division, LP, Wins/Losses, Winrate).
     - Implement `/lol matches [member]`: Recent match history (Champion, K/D/A, Outcome, Map/Mode, Items, CS, Timestamp).
     - Implement `/lol mastery [member]`: Top champion masteries (Champion name, level, mastery points) and total mastery score.
   - **Trilingual Localization (`src/i18n.rs`)**:
     - Add full translations across English (EN), Vietnamese (VI), and Japanese (JA) for all new status, error, and LoL command messages.

## User Stories

1. As a Guild member, I want to run `/valorant status [region]` to see if the VALORANT servers are operational or under maintenance before queueing.
2. As a Guild member or developer using a personal API key, when I run `/valorant profile` or `/valorant matches` and receive a 403 Forbidden, I want a clear localized message explaining Riot API key tier limits rather than a cryptic error.
3. As a Guild member with a linked Riot account, I want to run `/lol profile` to view my summoner level, profile icon, and Solo/Duo and Flex ranks with winrate.
4. As a Guild member, I want to run `/lol matches` to see my recent match performances including champion played, outcome (Victory/Defeat), KDA, CS, items, and game mode.
5. As a Guild member, I want to run `/lol mastery` to see my highest champion masteries, champion levels, mastery points, and overall mastery score.
6. As a Guild member, I want to inspect another member (`/lol profile @member`, `/lol matches @member`, `/lol mastery @member`) only if they have granted consent via Guild Profile Visibility.
7. As a privacy-conscious user, I want my LoL data hidden from others by default until I enable visibility in the specific guild.
8. As a non-English speaker, I want all `/lol` and `/valorant status` messages in my Guild's configured language (English, Vietnamese, or Japanese).

## Architecture & Seams

- Domain port: `LolApiClient` trait.
- Domain engine: `LolService` taking `SqlitePool` and `Arc<dyn LolApiClient>`.
- Storage reuse: `valorant_linked_account` and `valorant_guild_visibility` tables via existing storage helpers.
- Presentation adapter: `src/commands/lol.rs` and `src/commands/valorant.rs` as pure leaf adapters.
- Golden rules check: 0 raw SQL in commands, 0 inverted dependencies (`grep commands:: src/app.rs src/handlers/`).
