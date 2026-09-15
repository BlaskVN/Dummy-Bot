# 04: League of Legends Slash Commands & Trilingual Localization

**What to build:**
1. Implement `/lol` slash command suite in `src/commands/lol.rs`:
   - `/lol profile [member]`:
     - Summoner Level, Profile Icon URL/ID, Solo/Duo & Flex Rank (Tier, Division, LP, Wins, Losses, Winrate %).
   - `/lol matches [member]`:
     - Recent match history (Champion, K/D/A, CS, Outcome badge, Mode, Items, Timestamp).
   - `/lol mastery [member]`:
     - Top champion masteries (Champion name, Level, Points) and Total Mastery Score.
2. Wire `LolService` into `Data` in `src/state.rs` and initialize client in `src/app.rs`.
3. Register `/lol` command in `src/commands/mod.rs` with full parameter/command descriptions in English, Vietnamese, and Japanese.
4. Add all translation keys and trilingual strings (EN, VI, JA) in `src/i18n.rs`.
5. Pre-commit architecture gate verification:
   - 0 inverted dependencies (`grep commands:: src/app.rs src/handlers/`).
   - 0 raw SQL in commands.
   - `cargo clippy --all-targets --all-features -- -D warnings`.
   - `cargo test`.

**Blocked by:** 01, 02, 03

**Status:** resolved

- [x] Implement `src/commands/lol.rs`.
- [x] Add `lol_api` and `lol_service` to `Data` in `src/state.rs` & wire in `src/app.rs`.
- [x] Register command and localized descriptions in `src/commands/mod.rs`.
- [x] Add trilingual translations in `src/i18n.rs`.
- [x] Verify test suite and pre-commit checks pass.
