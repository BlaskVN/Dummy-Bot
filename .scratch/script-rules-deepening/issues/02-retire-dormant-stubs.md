# Retire Dormant Script Stubs and Leaky Bindings

Status: resolved

## Overview
Delete unused `.rhai` modules and remove raw database/discord bindings from `src/core/bindings/`.

## Details
- Remove `modules/moderation.rhai`, `modules/attendance.rhai`, `modules/community.rhai`, `modules/word_puzzle.rhai`.
- Remove `src/core/bindings/database.rs` and `src/core/bindings/discord.rs`.
- Keep clean logger bindings for rule diagnostics.
