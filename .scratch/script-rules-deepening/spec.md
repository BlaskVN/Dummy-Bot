# Spec: Deepen Scripting Engine into Dedicated Rules Adapter

## Problem Statement
The scripting layer (`src/core/`) exposes raw database and Discord bindings across a wide seam:
1. Four of five modules in `modules/` (`moderation.rhai`, `attendance.rhai`, `community.rhai`, `word_puzzle.rhai`) are disconnected stubs never called by the bot.
2. Raw JSON string/parse database bindings and discord formatting bindings leak across the engine interface into scripts.
3. Callers (`CoreEventBus`) interact with raw `rhai::Map` dynamic return values, requiring ad-hoc extraction and coupling caller logic to Rhai-specific internals.
4. There is no substitution seam to swap or unit-test rule evaluation without filesystem scripts.

## Goals & Architecture
We deepen the scripting engine into a dedicated rule evaluation adapter (`RuleEngine` seam):
- **Domain Typed Interface**:
  - `RuleContext`: `content`, `author_id`, `guild_id`.
  - `RuleDecision`: `Pass`, `FlagSuggestion { rule_id, reason, should_warn }`.
  - Trait `RuleEngine`: `evaluate_content(&self, ctx: &RuleContext) -> Result<RuleDecision>`, `reload(&self) -> Result<()>`.
- **Deep Adapter `RhaiRuleEngine`**:
  - Encapsulates AST caching, hot reloading, and bounded execution budget (`max_operations: 100_000`, `max_call_levels: 50`).
  - Provides a sandboxed Rhai environment with logger bindings (`log_info`, `log_warn`, `log_error`).
  - Safely translates Rhai return structures into domain `RuleDecision` instances.
  - Eliminates raw database (`database.rs`) and Discord formatting (`discord.rs`) bindings.
- **Retire Dormant Script Stubs**:
  - Remove `modules/moderation.rhai`, `modules/attendance.rhai`, `modules/community.rhai`, and `modules/word_puzzle.rhai`.
  - Keep `modules/automod.rhai` as the dedicated rule script.
- **Integration**:
  - `CoreEventBus` calls `rule_engine.evaluate_content(...)` via the typed seam.
  - `/reload_modules` slash command triggers `rule_engine.reload()`.

## Success Criteria
1. Dormant `.rhai` files removed from `modules/`.
2. Leaky database and discord bindings removed.
3. Callers interact strictly with `RuleContext` and `RuleDecision`.
4. Sandboxing guarantees budget limits.
5. All tests pass with full coverage for `RhaiRuleEngine` and mock rule evaluation.
