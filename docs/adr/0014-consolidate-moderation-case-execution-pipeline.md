# Consolidate Moderation Case Execution Pipeline

Moderation slash commands (`warn`, `kick`, `ban`, `timeout`) previously duplicated authorization checks, hierarchy denial evaluations, Discord action dispatching, database case creation, summary rendering, and error logging across 4 separate handler implementations.

We consolidated the execution flow behind a single deep pipeline entrypoint `execute_moderation_pipeline` in `commands/moderation/mod.rs` and deepened `execute_moderation_action` in `moderation_cases.rs` to encapsulate denial verification.

This provides:
1. Complete locality for moderation authorization and case creation.
2. Individual moderation commands (`warn`, `kick`, `ban`, `timeout`) become ultra-thin shims responsible solely for CLI parameter parsing.
3. Consistent error handling and response rendering across all moderation actions.
