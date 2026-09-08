# Define RuleEngine Seam and Typed Domain Models

Status: resolved

## Overview
Define `RuleEngine` trait, `RuleContext`, and `RuleDecision` domain models, and implement the deep `RhaiRuleEngine` adapter with AST caching, bounded budget, and typed result mapping.

## Details
- Create domain types in `src/core/models.rs` or `src/core/mod.rs`.
- Implement `RuleEngine` trait and `RhaiRuleEngine` in `src/core/engine.rs`.
- Add unit tests verifying `Pass`, `FlagSuggestion`, and bounded operation budget timeouts.
