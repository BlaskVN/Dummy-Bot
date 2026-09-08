# Real Riot Account Lookup & API Error Resilience

Status: closed
Blocked by: 04

## Overview
Enhance Riot API integration to query official Riot Account-V1 API during `/valorant link` to retrieve and verify real PUUIDs and casing, handle 403 Forbidden restrictions from Riot Personal Developer keys gracefully with clear user-facing explanations, and eliminate unhandled errors in `/valorant profile` and `/valorant leaderboard`.

## Details
- Add `get_account_by_riot_id` to `RiotApiClient` trait, `HttpRiotApiClient`, and `MockRiotApiClient`.
- Support regional cluster routing for Account-V1 (`asia`, `americas`, `europe`).
- In `/valorant link`, verify account existence via Riot API and store verified PUUID, Game Name, and Tag Line. Reject unverified accounts with clean error feedback.
- In `HttpRiotApiClient::get_player_ranked`, return structured `RiotApiError::Forbidden` on 403 and `RiotApiError::NotFound` on 404 without string matching or synthesizing fake rank data in production.
- In `/valorant profile` and `/valorant leaderboard`, handle API failures gracefully with user-facing error messages instead of bubbling unhandled errors.
- Add unit tests for `get_account_by_riot_id` and error resilience.
- Run full CI checks (`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`).
