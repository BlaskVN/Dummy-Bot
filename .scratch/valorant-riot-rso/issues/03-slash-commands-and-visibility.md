# VALORANT Slash Commands & Guild Visibility

Status: resolved
Blocked by: 01, 02

## Overview
Implement `/valorant` slash subcommands for profile lookup, Guild leaderboard, visibility management, linking, and unlinking, with trilingual localizations and documentation updates.

## Details
- Implement `/valorant profile [member]` with privacy checks for non-self lookups.
- Implement `/valorant leaderboard` ranking visible Guild members.
- Implement `/valorant visibility <enable|disable|status>`.
- Implement `/valorant link <riot_id> [region]` and `/valorant unlink`.
- Add translations across English, Vietnamese, and Japanese in `src/i18n.rs` and `src/commands/mod.rs`.
- Update `CONTEXT.md` and `docs/adr/0004-use-riot-rso-for-valorant-player-data.md` to remove Tracker Profile Link.
- Add command structure unit tests and parameter localization tests.
