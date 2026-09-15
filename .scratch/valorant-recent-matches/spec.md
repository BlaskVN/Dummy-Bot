# Spec: VALORANT Recent Matches & Voice Domain Seam Encapsulation

## Problem Statement

While Dummy Bot v3.3.0 supports linked Riot accounts, official rank display (`/valorant profile`), and Guild VALORANT Leaderboards (`/valorant leaderboard`), it currently lacks recent match history display (`recent matches`), which is an explicit deliverable under the **Riot-approved milestone** in `docs/ROADMAP.md`. Guild members who link their Riot account cannot inspect their latest performance (agents, maps, match outcomes, scores, and K/D/A) directly in Discord. Furthermore, inspecting others requires strict enforcement of **Guild Profile Visibility** to prevent unconsented surveillance and maintain compliance with Riot Games policies and ADR-0004 / ADR-0005.

Additionally, architectural analysis of the codebase reveals an inverted dependency smell: `src/handlers/reconnect.rs` imports `crate::commands::voice::update_voice_state`. According to `.agents/skills/deep-feature/SKILL.md` (Golden Rule 1: Presentation is a Pure Leaf Adapter), slash commands must never be imported into handlers or background tasks; voice state management belongs in a dedicated domain module (`src/voice.rs`).

## Solution

1. **Voice Domain Seam**: Encapsulate Discord voice connection state updates in a deep domain module `src/voice.rs`, making `src/commands/voice.rs` a thin presentation adapter and eliminating the inverted dependency in `src/handlers/reconnect.rs`.
2. **VALORANT Recent Matches**:
   - Extend the `RiotApiClient` seam with `get_recent_matches(region, puuid, count)`.
   - Provide realistic mock match history in `MockRiotApiClient` for offline/deterministic testing.
   - Implement official VAL-MATCH-V1 API integration in `HttpRiotApiClient`.
   - Add `get_recent_matches` to `ValorantService` with strict privacy enforcement: viewing another member requires that member to have enabled **Guild Profile Visibility** in the current Guild.
   - Provide `/valorant matches [member]` slash command formatted with rich embeds and trilingual localization (EN, VI, JA).

## User Stories

1. As a Guild member with a linked Riot account, I want to run `/valorant matches` so that I can see a summary of my recent competitive and unrated games.
2. As a Guild member, I want to see the agent played, map name, round score (won/lost), game mode, K/D/A, and match outcome for each recent match so that I can review my performance.
3. As a Guild member without a linked Riot account, I want `/valorant matches` to inform me that I have not linked my account and guide me to link it.
4. As a Guild member, I want to run `/valorant matches @member` to view another member's recent matches when they have enabled Guild Profile Visibility.
5. As a privacy-conscious Guild member, I want my recent matches to remain hidden from other members by default until I explicitly enable Guild Profile Visibility in that Guild.
6. As a Guild member, I want an informative, non-leaking message when I attempt to view another member's matches who has not enabled visibility in the Guild.
7. As a Guild member, I want match timestamps and dates formatted clearly and cleanly in Discord embeds.
8. As an English, Vietnamese, or Japanese speaking user, I want all match history outputs and error messages localized in my Guild's language.
9. As a developer, I want to test the entire recent matches pipeline in-memory using deterministic mock data without hitting live Riot APIs or needing API keys.
10. As a system operator, I want gateway reconnections and shard restarts to restore voice connections without relying on presentation layer command modules.

## Implementation Decisions

- **Domain Model (`RecentMatchSummary`)**:
  Placed in `src/valorant/riot_api.rs`. Fields include `match_id`, `map_name`, `game_mode`, `game_start_millis`, `character`, `rounds_won`, `rounds_lost`, `kills`, `deaths`, `assists`, `won`.
- **RiotApiClient Seam**:
  Extend `RiotApiClient` trait with `get_recent_matches<'a>(&'a self, region: RiotRegion, puuid: &'a str, count: usize) -> BoxFuture<'a, Result<Vec<RecentMatchSummary>>>`.
- **Mock Implementation**:
  `MockRiotApiClient` generates deterministic recent matches representing various agents (Jett, Omen, Killjoy), maps (Ascent, Haven, Lotus), and outcomes (victory, defeat).
- **Service Layer (`ValorantService`)**:
  `get_recent_matches(&self, requester_user_id: u64, target_user_id: u64, guild_id: GuildId, count: usize) -> Result<PlayerRecentMatchesData, ValorantMatchesError>`.
  Enforces `NotLinked`, `HiddenOther`, and maps domain errors.
- **Presentation Layer (`src/commands/valorant.rs`)**:
  Register `/valorant matches` subcommand with optional `member` parameter. Formats match cards into Discord embed with colored status indicators (Green for win, Red for loss).
- **Voice Seam (`src/voice.rs`)**:
  Move `update_voice_state` into `src/voice.rs`. `src/commands/voice.rs` and `src/handlers/reconnect.rs` both call `crate::voice::update_voice_state`.
- **Localization (`src/i18n.rs`)**:
  Add keys: `ValorantMatchesTitle`, `ValorantMatchesEmpty`, `ValorantMatchWon`, `ValorantMatchLost`, `ValorantMatchScore`, `ValorantMatchKda`, `ValorantMatchesHiddenOther`, `ValorantMatchesNotLinkedSelf`, `ValorantMatchesNotLinkedOther`.

## Testing Decisions

- **Seams Under Test**:
  1. `voice` seam: test that `update_voice_state` sends appropriate gateway payloads without calling into commands.
  2. `RiotApiClient` seam: verify `MockRiotApiClient` and `HttpRiotApiClient` deserialization and error mappings.
  3. `ValorantService` seam: verify self-view, other-view with visibility enabled, other-view with visibility disabled, and unlinked scenarios.
  4. Pre-commit architectural verification: verify 0 inverted dependencies (`grep commands:: src/app.rs src/handlers/`) and 0 raw SQL queries in commands/handlers.

## Out of Scope

- Opponent scouting or alternative MMR/ELO calculations (explicitly forbidden by ADR-0004 and Riot Developer Terms).
- Weapon skins, economy graphs, or detailed round-by-round replay analytics.
- Web dashboard match views (minimal companion website covers only legal/RSO auth flows).

## Further Notes

This work fulfills the remaining requirement of the **Riot-approved milestone** in `docs/ROADMAP.md` while maintaining complete consistency with `CONTEXT.md` and ADR-0004 / ADR-0005.
