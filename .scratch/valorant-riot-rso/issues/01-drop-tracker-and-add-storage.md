# Drop Tracker Profile & Implement Linked Account Storage

Status: resolved

## Overview
Remove the legacy `valorant_tracker_profile` table and establish schema & storage methods for `LinkedRiotAccount` and per-Guild `GuildProfileVisibility`.

## Details
- Add migration `0025_valorant_riot_accounts.sql` to drop `valorant_tracker_profile` and create `valorant_linked_account` and `valorant_guild_visibility`.
- Implement `src/valorant/storage.rs`:
  - `get_linked_account`, `set_linked_account`, `remove_linked_account`.
  - `get_guild_visibility`, `set_guild_visibility`, `list_guild_visible_accounts`.
- Add `valorant_guild_visibility` to `delete_guild_data` in `src/database.rs` for strict Guild Data isolation.
- Write unit tests for storage operations and Guild isolation.
