# Companion Website & Production Riot RSO Settings

Status: resolved
Blocked by: 03

## Overview
Add production Riot RSO environment configuration support to `src/config.rs` and `config.env`, and implement the Companion Website minimal web surface under `docs/` compliant with ADR-0004 and CONTEXT.md.

## Details
- Update `src/config.rs` to load optional `riot_rso_client_id`, `riot_rso_client_secret`, and `riot_rso_redirect_uri`.
- Document new environment variables in `config.env`.
- Add unit test for configuration parsing.
- Implement the Companion Website under `docs/`:
  - `docs/index.html`: Minimal bot info, legal links, RSO login explanation, and OAuth2 authorization button.
  - `docs/auth/callback/index.html` and `docs/auth/callback.html`: RSO OAuth2 callback handler, verification instructions, unlinking guidance, and data deletion request instructions.
  - `docs/privacy.html`: Privacy Policy detailing Riot player data handling and Guild Profile Visibility consent.
  - `docs/terms.html`: Terms of Service.
- Run full CI verification (`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`).

## Answer
- Config struct in `src/config.rs` updated with `riot_rso_client_id`, `riot_rso_client_secret`, and `riot_rso_redirect_uri`, with unit test `parses_riot_rso_config_vars`.
- `config.env` updated with documented Riot API and RSO environment variables.
- Companion Website implemented under `docs/` (`index.html`, `auth/callback/index.html`, `auth/callback.html`, `privacy.html`, `terms.html`) adhering strictly to the minimal web surface requirement defined in `CONTEXT.md` and `docs/adr/0004-use-riot-rso-for-valorant-player-data.md`.

