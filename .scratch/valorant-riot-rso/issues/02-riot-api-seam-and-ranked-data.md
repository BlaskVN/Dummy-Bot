# Riot API Seam & Player Ranked Data

Status: resolved
Blocked by: 01

## Overview
Extend the `RiotApiClient` seam to retrieve player competitive data (tier name, RR, act wins) with competitive scoring for leaderboards, implemented in both production HTTP client and deterministic mock client.

## Details
- Define `PlayerRankedData` with `tier_weight()` and `competitive_score()`.
- Add `get_player_ranked` method to `RiotApiClient` trait.
- Implement `HttpRiotApiClient::get_player_ranked` for Riot API VAL-RANKED endpoints.
- Implement `MockRiotApiClient::get_player_ranked` returning deterministic stats based on PUUID.
- Write unit tests for mock data generation, tier scoring, and serialization.
