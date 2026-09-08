# Spec: Consolidate Moderation Case Execution Pipeline

## Problem Statement
Moderation slash commands (`warn`, `kick`, `ban`, `timeout`) previously duplicated authorization checks, hierarchy denial evaluations, Discord action dispatching, database case creation, summary rendering, and error logging across 4 separate handler implementations.

## Goals & Architecture
Consolidate moderation actions behind a unified pipeline entrypoint:
- `execute_moderation_pipeline` in `src/commands/moderation/mod.rs` evaluates role hierarchy and self/bot targeting denials upfront and coordinates case creation.
- `execute_moderation_action` in `src/moderation_cases.rs` atomically performs Discord action execution, sequential case numbering, and moderation channel notice posting.
- Thin command shims for `warn`, `kick`, `ban`, and `timeout` focus solely on parameter parsing.

## Success Criteria
1. Single consolidated moderation execution path.
2. Zero duplicate case allocation or channel notice posting logic.
3. 100% test coverage for success, self-target denials, hierarchy denials, and database isolation.
