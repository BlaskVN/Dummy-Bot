# Deepen Scripting Engine into Dedicated Rules Adapter

The scripting layer historically exposed raw database and Discord bindings across a wide seam, while four of five `.rhai` modules in `modules/` remained dormant stubs never executed by bot workflows. Furthermore, callers directly managed Rhai-specific dynamic map types without domain type safety.

We deepened the scripting engine into a dedicated rule evaluation adapter (`RuleEngine` trait and `RhaiRuleEngine` implementation). Dormant modules (`moderation.rhai`, `attendance.rhai`, `community.rhai`, `word_puzzle.rhai`) and leaky database bindings were retired.

This provides:
1. High locality and leverage: AST compilation, hot reloading, sandboxed budget limits, and result parsing are encapsulated inside `RhaiRuleEngine`.
2. Type safety: callers receive domain-typed `RuleDecision` enum variants (`Pass`, `FlagSuggestion`) without Rhai dependency.
3. Testability: callers can be tested against the `RuleEngine` seam using in-memory doubles without filesystem access.
