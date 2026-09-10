# 04: Add /valorant matches Slash Command and Trilingual Localization

**What to build:**
Register `/valorant matches [member]` in `src/commands/valorant.rs`. Render an embed presenting recent matches with agent, map, game mode, round score, KDA, win/loss color indicators, and timestamp. Add trilingual localization keys in `src/i18n.rs` (English, Vietnamese, Japanese).

**Blocked by:** 03 (Implement ValorantService get_recent_matches with Privacy Enforcement)

**Status:** complete

- [x] Add translation keys to `src/i18n.rs` (`ValorantMatchesTitle`, `ValorantMatchesEmpty`, `ValorantMatchWon`, `ValorantMatchLost`, `ValorantMatchScore`, `ValorantMatchKda`, `ValorantMatchesHiddenOther`, `ValorantMatchesNotLinkedSelf`, `ValorantMatchesNotLinkedOther`) in EN, VI, JA.
- [x] Implement `matches` slash command in `src/commands/valorant.rs`.
- [x] Format match summaries cleanly into a Poise/Serenity embed.
- [x] Verify `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`.
