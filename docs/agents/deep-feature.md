# Deep Feature Development & Architecture Guidelines

Guidelines for building features **deep-first**. Place substantial domain behavior and persistence behind small, testable interfaces at clean seams, keeping presentation layers thin and free of business logic.

This document synthesizes all architectural lessons learned across the Dummy-Bot codebase to eliminate the need for subsequent architecture cleanup loops.

---

## The 5 Golden Rules (Architectural Contract)

### 1. Presentation is a Pure Leaf Adapter (No Inverted Dependencies)
- `src/commands/*` modules are strictly consumer adapters for Discord UI/CLI.
- **NEVER** import `crate::commands::*` into `src/app.rs`, `src/handlers/*`, or background tasks.
- If a background loop, gateway reconnect, or startup hook needs to perform an action (e.g. restoring presence, reconciling mini-games, expiring sessions), that lifecycle logic **must** live in a dedicated domain module (e.g. `src/presence.rs`, `src/word_puzzle_engine.rs`), never inside a slash command.

### 2. Zero Raw SQL in Handlers and Presentation
- Handlers (`src/handlers/*`) and slash commands (`src/commands/*`) must **never** execute raw `sqlx::query*`.
- Every database entity and aggregate belongs to a domain module that encapsulates queries, updates, and transaction boundaries.
- Handlers and commands only:
  1. Extract and validate user arguments.
  2. Delegate to the domain module.
  3. Map domain error enums to user-facing localized `TranslationKey`.
  4. Format the Discord embed/response.

### 3. Atomic Lifecycle Pipelines (No Scattered Multi-Step Coordination)
- Whenever an entity transitions states (creation, completion, cancellation, deletion, expiry), provide **one atomic pipeline function** in the domain module.
- Never force callers across different handlers to manually orchestrate a multi-step checklist (e.g., `pause_attendance` $\to$ `finalize_aggregates` $\to$ `reconcile_rewards` $\to$ `clear_presence`).
- If 4 callers handle an event ending, all 4 must call `domain::terminate_*` rather than reimplementing the teardown steps.

### 4. Decouple External I/O via Ports & Payloads
- Domain engines must remain independent of Discord framework types (`serenity::Context`, `serenity::ChannelId`, `poise::Context`).
- Define lightweight domain payload structs and port traits (e.g., `WordPuzzleOutbox`, `RewardRoleAssigner`).
- Domain engines take the port trait (`outbox: &O`), while production code supplies the Discord adapter (e.g., `DiscordOutbox { ctx, data }`).
- This guarantees 100% of business logic, retry policies, and edge cases can be tested in-memory with simple test doubles without Discord mocks.

### 5. Strict Domain Model & Vocabulary Alignment (`CONTEXT.md`)
- Every struct, enum, function, variable, and doc comment must match terms in `CONTEXT.md` exactly:
  - Say **Guild**, never "server".
  - Say **Guild Member**, never generic "user" for guild operations.
  - Say **Session Credit** and **Play Time**, never "XP" or "Reputation".
  - Say **Word Puzzle Session**, never "Wordle".
  - Say **Activity Reward Role** and **Bot-owned Reward Role**, never "Permission Role" or "Existing Reward Role".
- Use strongly-typed domain enums (`BotStatus`, `ActivityKind`, `LetterMark`) over string primitives. Convert to/from database strings at the storage seam only.

---

## The 4-Phase Implementation Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Phase 1: Seam & Domain Type Design                         │
│  - Check CONTEXT.md for exact terms                         │
│  - Design small interface + domain enums (no framework I/O) │
│  - Apply Deletion Test (does complexity concentrate?)       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 2: TDD at the Seam (Red -> Green)                    │
│  - In-memory SQLite tests (`sqlite::memory:`)               │
│  - Mock ports for message delivery / external APIs          │
│  - Verify lifecycle, retries, idempotency, isolation        │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 3: Thin Presentation & Handler Wiring                │
│  - Wire slash commands as thin parameter/embed shims        │
│  - Wire gateway/daemon hooks into domain functions          │
│  - Map domain errors to TranslationKey (no raw DB errors)   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 4: Architecture Verification Gate (Pre-Commit)       │
│  - Inverted dependency check: grep "commands::" in src/     │
│  - Raw SQL check: grep "sqlx::query" in commands & handlers │
│  - Cargo clippy clean (-D warnings)                         │
│  - Cargo test clean (100% passing)                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Practical Patterns

### Pattern A: Decoupled Outbox Port (Mini-games / Announcements)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPayload<'a> {
    pub guild_id: &'a str,
    pub channel_id: &'a str,
    pub content: &'a str,
}

#[async_trait::async_trait]
pub trait NotificationOutbox: Send + Sync {
    async fn deliver(&self, payload: NotificationPayload<'_>) -> Result<(), anyhow::Error>;
}
```

### Pattern B: Atomic Lifecycle Teardown

```rust
// In domain module: src/community.rs
pub async fn terminate_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    now: i64,
) -> Result<()> {
    crate::attendance::pause_session(pool, guild_id, event_id, now).await?;
    crate::activity_aggregate::finalize_activity(pool, guild_id, event_id, now).await?;
    Ok(())
}
```

### Pattern C: Typed Domain Record at Storage Seam

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotPresenceRecord {
    pub status: BotStatus,
    pub activity_kind: Option<ActivityKind>,
    pub activity_text: Option<String>,
}
```

---

## Pre-Commit Verification Gate

Run these 4 automated checks before opening a PR:

1. **Inverted Dependency Scan**:
   ```bash
   grep -rn "commands::" src/app.rs src/handlers/
   ```
   *(Expected: 0 matches)*

2. **Raw SQL Leak Scan**:
   ```bash
   grep -rn "sqlx::query" src/commands/ src/handlers/
   ```
   *(Expected: 0 matches)*

3. **Compiler & Linter Gate**:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
   *(Expected: 0 errors, 0 warnings)*

4. **Test Suite Gate**:
   ```bash
   cargo test
   ```
   *(Expected: 100% passing)*
