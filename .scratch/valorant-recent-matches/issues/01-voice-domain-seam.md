# 01: Encapsulate Discord Voice State Updates in Domain Seam

**What to build:**
Extract `update_voice_state` out of presentation module `src/commands/voice.rs` into domain module `src/voice.rs`. Update `src/handlers/reconnect.rs` and `src/commands/voice.rs` to consume `crate::voice::update_voice_state`, eliminating the inverted dependency smell (`reconnect.rs -> commands::voice`).

**Blocked by:** None (can start immediately).

**Status:** complete

- [x] Create `src/voice.rs` with `pub fn update_voice_state(ctx: &serenity::Context, guild_id: serenity::GuildId, channel_id: Option<serenity::ChannelId>)`.
- [x] Refactor `src/commands/voice.rs` to delegate to `crate::voice::update_voice_state`.
- [x] Refactor `src/handlers/reconnect.rs` to import from `crate::voice` instead of `crate::commands::voice`.
- [x] Verify `grep -rn "commands::" src/app.rs src/handlers/` returns 0 matches (except app.rs command registration).
