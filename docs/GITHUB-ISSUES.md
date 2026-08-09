# GitHub Kanban backlog

Tài liệu này chứa các issue có thể sao chép trực tiếp vào GitHub cho năm milestone đã tạo. Issues được đánh số kế hoạch liên tục từ `#1` đến `#47`; hãy tạo theo đúng thứ tự này. GitHub tự cấp số trên toàn repository, nên nếu repository đã có issue/PR mang số trước đó thì phải thay các số trong `Blocked by` bằng số thực tế sau khi tạo.

## Quy ước chung

- Project status ban đầu: `Backlog`.
- Priority: `P0` chặn release, `P1` cần có trong release, `P2` có thể làm sau các issue P0/P1 cùng milestone.
- Mọi command mới là slash command, dùng quyền native của Discord và chỉ hoạt động trong Guild khi issue ghi Guild-scoped.
- Mọi dữ liệu Guild phải luôn được truy vấn cùng `guild_id`; dữ liệu toàn cục phải được ghi rõ là toàn cục.
- Mọi chuỗi hiển thị mới phải có EN, VI và JA; `src/i18n.rs` hiện đã hỗ trợ cả ba ngôn ngữ.
- Mỗi migration chỉ tiến về phía trước và phải chạy được trên database v1.0 hiện có (`migrations/0001_initial.sql`).
- Không tạo abstraction cho nhiều implementation, generic mini-game framework, repository layer tổng quát, scheduler tổng quát hay event bus nội bộ nếu issue không cần.
- Trước khi đóng issue: chạy test được nêu trong issue, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, và `cargo test --locked`.

## Nguồn kỹ thuật được phép dùng

- Discord Developer Documentation: [Gateway intents](https://docs.discord.com/developers/events/gateway#gateway-intents), [Gateway events](https://docs.discord.com/developers/events/gateway-events), [Permissions](https://docs.discord.com/developers/topics/permissions), [Auto Moderation](https://docs.discord.com/developers/resources/auto-moderation), [Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event), [Audit Log](https://docs.discord.com/developers/resources/audit-log), [Message](https://docs.discord.com/developers/resources/message).
- Discord user privacy behavior: [Activity Sharing FAQ](https://support.discord.com/hc/en-us/articles/7931156448919-Activity-Sharing-on-Discord-FAQ).
- VALORANT/Tracker policy boundary: [Riot VALORANT API documentation](https://developer.riotgames.com/docs/valorant), [Tracker Network response on a public VALORANT API](https://feedback.tracker.gg/t/development-of-a-bot-discord-stats-valorant/43663).
- Library surface actually installed in this repository: Poise `0.6.2`, Serenity `0.12.5`, SQLx `0.9.0`, Chrono `0.4.45` in `Cargo.lock`.

---

# v1.1 — Public foundation

## #1 — Persist and validate a Guild IANA time zone

**Priority:** P0  
**Labels:** `type:feature`, `area:configuration`, `area:database`

### Outcome

A Guild Administrator can set, view, or clear the time zone used for Guild-day boundaries. An unset value is explicit; no fixed-offset approximation is accepted.

### Scope

- Add a nullable time-zone field/table keyed by `guild_id` through a new migration.
- Add `/timezone set <iana_name>`, `/timezone show`, and `/timezone clear`, guarded by `MANAGE_GUILD`.
- Parse against the IANA database, including zones with daylight-saving transitions. Add the smallest Rust dependency that supplies the IANA database; Chrono alone only supplies timestamp/date-time primitives in the current build.
- Include the value or `Not configured` in `/settings`.

### Acceptance criteria

- [ ] `Asia/Bangkok` and another canonical IANA name are accepted and round-trip through SQLite.
- [ ] Unknown names and fixed offsets such as `UTC+7` are rejected without changing stored configuration.
- [ ] Clearing the value disables all later automation that requires a Guild-day boundary.
- [ ] Commands cannot read or modify another Guild's value.
- [ ] Responses exist in EN, VI and JA.

### Tests

- Unit test valid/invalid parsing and the next-05:00 calculation across at least one DST transition.
- Database test proving two Guilds retain different zones.

### Expected code areas

`migrations/`, `src/commands/configuration/`, `src/commands/configuration/settings.rs`, `src/i18n.rs`, `Cargo.toml`.

---

## #2 — Deliver one-time installation onboarding without requesting a new permission

**Priority:** P1  
**Labels:** `type:feature`, `area:onboarding`, `area:discord`

### Outcome

On first installation, the installer receives setup guidance by DM when identifiable; otherwise the bot uses the Guild system channel when possible.

### Scope

- Handle a genuinely new Guild separately from reconnect/cache hydration.
- Persist completion keyed by `guild_id` before or atomically with delivery attempt so reconnects cannot spam.
- If the bot already has `VIEW_AUDIT_LOG`, inspect `BOT_ADD` audit entries and match the entry target to the current bot user; do not request this permission as an installation requirement.
- Try the identified installer's DM first. On lookup/DM failure, try the configured system channel only when effective permissions include `VIEW_CHANNEL` and `SEND_MESSAGES`.

### Acceptance criteria

- [ ] Message mentions `/settings`, language/time-zone setup, and that automations are opt-in.
- [ ] No channel is created and no unrelated member is DM'd.
- [ ] Reconnect, resume and `GuildCreate { is_new: false }` do not resend onboarding.
- [ ] Failure of both destinations is logged and does not fail bot startup.
- [ ] Content exists in EN, VI and JA, using the Guild language for fallback channel delivery.

### Tests

- State test for new Guild, reconnect, audit-log unavailable, DM failure and system-channel failure.

### Docs basis

Discord exposes `BOT_ADD` in the [Audit Log event types](https://docs.discord.com/developers/resources/audit-log#audit-log-entry-object-audit-log-events); audit-log access requires `VIEW_AUDIT_LOG`.

### Expected code areas

`migrations/`, `src/handlers/mod.rs`, a focused onboarding handler, `src/permissions.rs`, `src/i18n.rs`.

---

## #3 — Store owner-managed Donation Information and an uploaded QR image

**Priority:** P2  
**Labels:** `type:feature`, `area:owner`, `area:database`

### Outcome

The Bot Owner can atomically replace or clear global Donation Information that survives restarts and expired Discord attachment URLs.

### Scope

- Add owner-only `/donation set` and `/donation clear` slash commands.
- `set` accepts an optional owner-supplied message, optional HTTPS URL and optional uploaded QR image; at least one value is required. Bot UI is localized, but the owner-supplied message is stored verbatim as one global value.
- Accept PNG/JPEG only, enforce the existing `attachment_max_bytes`, download through the existing bounded `reqwest` client/semaphore, and persist owned bytes or an owned local file rather than Discord's signed CDN URL.
- Donation configuration is global, not per Guild.

### Acceptance criteria

- [ ] Only configured `owners` can call either command.
- [ ] MIME type, PNG/JPEG file signature, size, URL scheme and empty update are validated before replacing the prior value.
- [ ] A failed download/validation preserves the previous complete configuration.
- [ ] `/donation clear` removes metadata and owned QR storage; it never deletes an arbitrary path.
- [ ] User-provided text is sent with allowed mentions disabled.

### Tests

- Validation tests for valid PNG/JPEG, wrong signature, oversized upload, non-HTTPS URL and atomic replacement failure.

### Expected code areas

`migrations/`, `src/commands/`, `src/state.rs`, existing attachment-download helpers, `src/i18n.rs`.

---

## #4 — Expose Donation Information through `/donate` and `/botinfo`

**Priority:** P2  
**Labels:** `type:feature`, `area:general`

**Blocked by:** #3

### Outcome

Users can request Donation Information without unsolicited donation messages.

### Acceptance criteria

- [ ] `/donate` shows configured text/link/QR and a localized `not configured` response otherwise.
- [ ] The QR is uploaded from the bot-owned stable copy, not embedded from a stale external URL.
- [ ] `/botinfo` contains only a short, non-promotional reference to `/donate` when donation data exists.
- [ ] No automatic donation posts, DMs, command-response footers or per-Guild overrides are added.
- [ ] EN, VI and JA output is covered.

### Tests

- Command rendering test for empty, text-only and QR configurations; verify allowed mentions are disabled.

### Expected code areas

`src/commands/general/botinfo.rs`, new donation command module, command registration in `src/commands/mod.rs`, `src/i18n.rs`.

---

## #5 — Configure a dedicated Moderation Channel

**Priority:** P0  
**Labels:** `type:feature`, `area:moderation`, `area:configuration`

### Outcome

Moderation cases and AutoMod observations have a destination separate from Message Log.

### Scope and acceptance criteria

- [ ] Add `/moderation-channel set <channel>`, `show`, and `clear`, guarded by `MANAGE_GUILD`.
- [ ] Accept a Guild text channel only and reject a channel from another Guild.
- [ ] Reuse `missing_channel_permissions`; require effective `VIEW_CHANNEL`, `SEND_MESSAGES` and `EMBED_LINKS` after overwrites.
- [ ] Persist by `guild_id` and display independently from Message Log in `/settings`.
- [ ] Clearing this setting does not alter `message_log_config`.
- [ ] EN, VI and JA responses exist.

### Tests

- Permission/validation unit tests and a two-Guild persistence test.

### Expected code areas

`migrations/`, `src/commands/configuration/`, `src/permissions.rs`, `src/i18n.rs`.

---

## #6 — Add immutable per-Guild Moderation Case storage

**Priority:** P0  
**Labels:** `type:feature`, `area:moderation`, `area:database`

### Outcome

The database can create and void auditable cases with a gap-free sequential number per Guild under concurrent commands.

### Data contract

Store: `guild_id`, per-Guild `case_number`, action (`warn|kick|ban|timeout`), target user ID, moderator user ID, reason, created timestamp, optional Discord evidence URL, status, and nullable void actor/reason/timestamp. Do not store copied message text, matched AutoMod text or attachments.

### Acceptance criteria

- [ ] A unique database constraint covers `(guild_id, case_number)`.
- [ ] Allocation and insertion happen in one SQLite transaction; concurrent creation cannot duplicate a number.
- [ ] Original action fields cannot be edited by the application API.
- [ ] Void is a one-way state transition requiring actor and non-empty reason; a second void is rejected idempotently.
- [ ] Evidence accepts only a Discord message URL whose Guild component equals the current `guild_id`.
- [ ] Purge and AutoMod are not valid case types.

### Tests

- Migration test from `0001_initial.sql`; concurrent numbering; cross-Guild same case number; valid/invalid evidence URL; void preservation.

### Architecture references

`docs/adr/0001-store-minimal-moderation-evidence.md`, `docs/adr/0009-keep-moderation-cases-immutable.md`.

---

## #7 — Add `/warn` and `/timeout` with native permission and hierarchy checks

**Priority:** P0  
**Labels:** `type:feature`, `area:moderation`, `area:commands`

**Blocked by:** #6

### Outcome

Moderators can warn or timeout a member and receive exactly one case for a successful operation.

### Acceptance criteria

- [ ] `/warn <member> <reason> [evidence]` requires `MODERATE_MEMBERS` and applies existing self/user/bot hierarchy denial checks.
- [ ] `/timeout <member> <duration> <reason> [evidence]` requires `MODERATE_MEMBERS`, validates Discord's supported timeout limit, and reuses hierarchy checks.
- [ ] Warn records the case as its action; it does not invent a native Discord punishment.
- [ ] Timeout records a case only after Discord confirms the member edit.
- [ ] API/permission failure creates no successful case and returns a localized error.
- [ ] When #5 is configured, send the created case summary without copied evidence content.

### Tests

- Authorization/hierarchy, invalid duration, failed Discord action, successful action and case-channel notification tests.

### Docs basis

Discord's [permissions reference](https://docs.discord.com/developers/topics/permissions#permissions-bitwise-permission-flags) defines `MODERATE_MEMBERS`; role hierarchy constrains which roles/members a bot can act on.

---

## #8 — Attach Moderation Cases to successful `/kick` and `/ban`

**Priority:** P0  
**Labels:** `type:feature`, `area:moderation`

**Blocked by:** #5, #6

### Outcome

Existing kick and ban commands preserve their current native checks and create one case only after Discord success.

### Acceptance criteria

- [ ] Add optional evidence URL using the same validator as #6.
- [ ] Do not duplicate `moderation_denial` or permission logic already used by the commands.
- [ ] A Discord API failure creates no case.
- [ ] A database failure after a successful Discord action is reported as a critical consistency error and logged with action/Guild/target identifiers; it must not claim the Discord action failed.
- [ ] Successful case summaries go to the configured Moderation Channel; message-log behavior is unchanged.
- [ ] `/purge` never creates a case.

### Tests

- Focused service/command tests for Discord failure, DB failure and success for both actions.

### Expected code areas

`src/commands/moderation/kick.rs`, `src/commands/moderation/ban.rs`, shared minimal case creation function.

---

## #9 — Add Guild-scoped case viewing, listing and voiding commands

**Priority:** P1  
**Labels:** `type:feature`, `area:moderation`, `area:commands`

**Blocked by:** #6

### Acceptance criteria

- [ ] `/case view <number>` returns original fields and void metadata when present.
- [ ] `/case list [member]` is bounded and paginated; newest cases appear first.
- [ ] `/case void <number> <reason>` requires `MANAGE_GUILD` and preserves the original record.
- [ ] A case number from another Guild is indistinguishable from a missing case.
- [ ] Member mentions and reasons are rendered with allowed mentions disabled.
- [ ] EN, VI and JA output exists.

### Tests

- Cross-Guild isolation, pagination boundary, nonexistent case, successful void and repeated void.

---

## #10 — Add opt-in Discord AutoMod observation and required Gateway intents

**Priority:** P1  
**Labels:** `type:feature`, `area:moderation`, `area:discord`

**Blocked by:** #5

### Outcome

A Guild Administrator can enable observation only when the bot can actually receive native Discord AutoMod events.

### Acceptance criteria

- [ ] Add `/automod-observer enable|disable|status`, guarded by `MANAGE_GUILD`.
- [ ] Verify Serenity's current `GatewayIntents::non_privileged()` includes `AUTO_MODERATION_EXECUTION` and `AUTO_MODERATION_CONFIGURATION`, keep both enabled, and add an intent regression assertion/test around the configured set.
- [ ] Route Serenity `FullEvent::AutoModActionExecution` and `FullEvent::AutoModRuleUpdate` in `src/handlers/mod.rs`.
- [ ] Verify/report that the bot needs `MANAGE_GUILD`; Discord sends AutoMod events only to bots holding that permission.
- [ ] Enabling requires a configured Moderation Channel; disabling preserves native Discord rules and deletes/archives no Discord configuration.
- [ ] Do not expose any AutoMod rule-create/edit/delete command.

### Tests

- Configuration isolation, missing channel/permission, enable/disable and event-routing tests.

### Docs basis

The [intent table](https://docs.discord.com/developers/events/gateway#list-of-intents) separates configuration events from execution events. [Gateway AutoMod events](https://docs.discord.com/developers/events/gateway-events#auto-moderation) require the bot to have `MANAGE_GUILD`.

---

## #11 — Record bounded AutoMod execution metadata and notify moderators

**Priority:** P1  
**Labels:** `type:feature`, `area:moderation`, `area:database`

**Blocked by:** #10

### Outcome

Each enabled Guild receives useful execution notifications and retains only the metadata needed for the seven-day threshold.

### Acceptance criteria

- [ ] Store Guild ID, member ID, rule ID and event timestamp; optional channel/message IDs may be stored for navigation.
- [ ] Never persist `content`, `matched_content` or `matched_keyword` from the Gateway payload.
- [ ] Ignore disabled Guilds, bot users and malformed/duplicate deliveries.
- [ ] Notify the Moderation Channel with rule/member/action identifiers and available jump link; do not create a Moderation Case.
- [ ] Prune execution rows older than seven days during writes or a bounded startup pass; no general scheduler is added.
- [ ] Delivery failure does not lose the threshold event and is logged without user content.

### Tests

- Duplicate handling, different Guild/member/rule keys, pruning boundary and no-content-persistence test.

### Docs basis

Discord's [execution payload](https://docs.discord.com/developers/events/gateway-events#auto-moderation-action-execution) provides `guild_id`, `rule_id`, `user_id` and action metadata; content fields are unnecessary here.

---

## #12 — Manage one open AutoMod review suggestion per member and rule

**Priority:** P1  
**Labels:** `type:feature`, `area:moderation`, `area:workflow`

**Blocked by:** #11

### Outcome

The third execution for the same `(guild, member, rule)` within a rolling seven-day window opens one non-binding suggestion. It stays deduplicated until explicitly handled or that exact native rule is updated.

### Acceptance criteria

- [ ] Open a suggestion on transition from two to three qualifying events, not on every later event.
- [ ] Persist suggestion state so restart cannot post it again.
- [ ] Add a moderator interaction/command to mark the suggestion handled; record handler and handled timestamp.
- [ ] `AutoModRuleUpdate` resolves only open suggestions with the same Guild and `rule_id`; updating another rule changes nothing.
- [ ] After resolution, a new suggestion requires three new qualifying executions occurring after the resolution timestamp and inside a new rolling seven-day window.
- [ ] Suggestions never punish a member or modify a native rule.
- [ ] A failed notification remains an open suggestion but records that delivery failed; retry is moderator-triggered or on status view, not an unbounded background loop.

### Tests

- Third-event transition; fourth event deduplication; manual resolution; same-rule update; different-rule update; three new post-resolution events.

---

## #13 — Delete all Guild-owned data only after permanent bot removal

**Priority:** P0  
**Labels:** `type:feature`, `area:privacy`, `area:database`

### Outcome

Permanent removal erases the tenant; a temporary Discord outage preserves it.

### Acceptance criteria

- [ ] Route Serenity `FullEvent::GuildDelete { incomplete, .. }`.
- [ ] If Discord marks the Guild unavailable, perform no deletion.
- [ ] Otherwise delete every row keyed by that `guild_id` in one transaction and delete only files recorded as owned by that Guild.
- [ ] Global donation data and global Tracker links are never deleted by Guild cleanup.
- [ ] Cleanup sends no Discord message and is idempotent.
- [ ] Reinstall starts with clean Guild defaults.

### Tests

- A fixture containing every Guild-owned table proves outage preservation, permanent deletion, other-Guild preservation and global-data preservation.

### Docs basis

Discord documents the `unavailable` distinction on [Guild Delete](https://docs.discord.com/developers/events/gateway-events#guild-delete). Architecture decision: `docs/adr/0002-delete-data-when-the-bot-leaves-a-guild.md`.

---

## #14 — Detect and expose Degraded Message Log state

**Priority:** P1  
**Labels:** `type:feature`, `area:logging`, `area:discord`

### Outcome

Message Log does not silently imply that empty content is complete when privileged Message Content access is unavailable.

### Acceptance criteria

- [ ] Add a deployment setting controlling whether `MESSAGE_CONTENT` is included in the Identify intents; document its upgrade/default behavior.
- [ ] Represent Message Log health as `disabled`, `healthy` or `degraded` per Guild.
- [ ] Enter degraded state when Message Log is enabled but the deployment did not request Message Content; do not infer state from an individual empty message.
- [ ] Continue logging metadata actually present; never fabricate missing before/after content or attachments.
- [ ] Send one localized warning on transition into degraded state and persist the transition notification across restart.
- [ ] After confirmed recovery, return to healthy; a later new degradation may warn once again.
- [ ] Show health in `/messagelog status` and `/settings`.

### Tests

- Healthy→degraded→restart→healthy→degraded state sequence and metadata-only rendering.

### Docs basis

Discord explains that `MESSAGE_CONTENT` changes the availability of content fields in the [Gateway intent documentation](https://docs.discord.com/developers/events/gateway#message-content-intent). Decision: `docs/adr/0008-use-slash-commands-while-retaining-optional-message-content.md`.

---

## #15 — Add a multi-Guild data-isolation regression suite

**Priority:** P0  
**Labels:** `type:test`, `area:database`, `area:privacy`

### Outcome

Every v1.1 Guild-scoped query is proven tenant-safe before public installation is promoted.

### Acceptance criteria

- [ ] Seed two Guilds with overlapping case numbers and distinct settings.
- [ ] Exercise time zone, moderation channel, cases, AutoMod observation/suggestions and Message Log health through their public service/command query paths.
- [ ] Assert list, view, update, void and cleanup operations never read or mutate the second Guild.
- [ ] Assert global donation information remains shared and unaffected by Guild deletion.
- [ ] Test adds no production abstraction solely for mocking.

### Tests

- Run the new integration suite against a fresh migrated SQLite database and against a database upgraded from `0001_initial.sql`.

**Blocked by:** #1, #5, #6, #12, #13, #14

---

# v1.2 — Community activities

## #16 — Persist the bot extension for a VOICE Community Activity

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:database`

### Outcome

The bot stores only its domain additions while Discord Scheduled Event remains authoritative for schedule, voice location, visibility and lifecycle.

### Data contract

Store `guild_id`, Discord scheduled-event ID, host user ID, kind (`community|game`), optional game key, optional positive capacity, lifecycle state and notification marker. Do not copy the event name, description, start time or channel as a second source of truth.

### Acceptance criteria

- [ ] `(guild_id, scheduled_event_id)` is unique.
- [ ] Only `VOICE` entity type is accepted in v1.2; `EXTERNAL` and `STAGE_INSTANCE` are rejected as unsupported.
- [ ] Capacity is null/unlimited or a positive integer.
- [ ] Host is application-owned metadata because the bot—not the invoking member—is the API creator seen by Discord.
- [ ] All reads are Guild-scoped.

### Tests

- Migration, entity-type validation, capacity validation, unique identity and cross-Guild isolation.

### Docs basis

[Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event) defines entity types, fields and lifecycle. Decision: `docs/adr/0003-use-discord-scheduled-events-for-community-activities.md`.

---

## #17 — Create a VOICE Community Activity and its Discord Scheduled Event

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:discord`

**Blocked by:** #16

### Acceptance criteria

- [ ] `/activity create` collects name, start time, voice channel, optional description and optional capacity.
- [ ] Invoker must have effective `CREATE_EVENTS`, `VIEW_CHANNEL` and `CONNECT` in the selected voice channel.
- [ ] Validate Discord field limits before the HTTP request.
- [ ] Create a `VOICE` Scheduled Event through Serenity, then persist extension/host using the returned event ID.
- [ ] If local persistence fails after Discord creation, make one compensating delete attempt and clearly report any orphan; never silently claim success.
- [ ] Return a link/identifier for the native event and localized Join/Leave controls.
- [ ] No custom reminder scheduler or copied calendar row is created.

### Tests

- Permission matrix, invalid field/capacity, Discord failure, DB failure compensation and success.

### Docs basis

Discord's [VOICE event permission requirements](https://docs.discord.com/developers/resources/guild-scheduled-event#permissions-for-events-with-entity-type-voice) require `CREATE_EVENTS`, `VIEW_CHANNEL` and `CONNECT` for creation.

---

## #18 — View, update and cancel a managed Community Activity

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:commands`

**Blocked by:** #17

### Authorization contract

The stored Host may manage their activity through the bot; a member with `MANAGE_EVENTS` may manage any bot-managed activity. This application check is required because the native Scheduled Event was created through the bot account.

### Acceptance criteria

- [ ] `/activity view` fetches current native event data from Discord and combines it with bot extension data.
- [ ] `/activity update` validates Host-or-`MANAGE_EVENTS`, native bot permissions, allowed status transition and Discord field limits.
- [ ] `/activity cancel` cancels/deletes the native event and records terminal state idempotently.
- [ ] Commands refuse native events not registered as bot-managed activities.
- [ ] A missing/deleted native event is handed to reconciliation rather than recreated.
- [ ] Discord errors never overwrite local state as if successful.

### Tests

- Host, unrelated member and `MANAGE_EVENTS` authorization; invalid transition; missing event; repeated cancel.

---

## #19 — Persist confirmed participants and FIFO waitlist positions

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:database`

**Blocked by:** #16

### Outcome

Bot confirmation—not Discord subscription—determines roster membership.

### Acceptance criteria

- [ ] Store one membership row per activity/member with state `participant|waitlisted`, an insertion sequence and timestamps.
- [ ] Enforce uniqueness for one member per activity.
- [ ] FIFO order uses an immutable database sequence/timestamp with a deterministic tie-breaker, not Discord user ID alone.
- [ ] Bot accounts cannot be inserted.
- [ ] Native Scheduled Event subscriber/user events do not create or remove roster rows.

### Tests

- Unique membership, stable FIFO ordering, bot rejection and Guild scoping.

### Docs basis

Discord calls Scheduled Event users `subscribed` users in [Get Guild Scheduled Event Users](https://docs.discord.com/developers/resources/guild-scheduled-event#get-guild-scheduled-event-users); this project deliberately treats that as interest only.

---

## #20 — Implement idempotent Join and Leave interactions

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:interactions`

**Blocked by:** #19

### Acceptance criteria

- [ ] Join reserves a participant place when capacity is unlimited or not full; otherwise it creates a waitlist entry.
- [ ] Repeated Join returns current state without a duplicate row or changed FIFO position.
- [ ] Leave removes the caller's current membership; repeated Leave is a harmless localized `not joined` response.
- [ ] Join is closed for completed/canceled/missing activities.
- [ ] Concurrent joins for the final capacity slot result in exactly one participant and one waitlisted member.
- [ ] Responses are private/ephemeral where Discord supports it and exist in EN, VI and JA.

### Tests

- Unlimited capacity, exact capacity, repeated interaction, concurrent final slot and terminal activity.

---

## #21 — Promote the oldest waitlisted member after a place opens

**Priority:** P0  
**Labels:** `type:feature`, `area:community`, `area:workflow`

**Blocked by:** #20

### Acceptance criteria

- [ ] Leaving a full finite-capacity activity promotes exactly the oldest waitlisted member in the same transaction.
- [ ] Capacity increases promote as many FIFO entries as new places allow; capacity decreases never eject existing participants.
- [ ] Promotion notification is attempted once and persisted as delivered/failed; a DM failure does not roll back membership.
- [ ] Concurrent leaves cannot promote the same member twice or exceed capacity.
- [ ] Unlimited capacity has no waitlist after reconciliation.

### Tests

- Single/multiple promotion, capacity increase/decrease, concurrent leaves and failed notification.

---

## #22 — Reconcile native Scheduled Event update/delete lifecycle

**Priority:** P1  
**Labels:** `type:feature`, `area:community`, `area:discord`

**Blocked by:** #18, #21

### Acceptance criteria

- [ ] Verify Serenity's current `GatewayIntents::non_privileged()` includes `GUILD_SCHEDULED_EVENTS`, keep that intent enabled, and route create/update/delete events used by reconciliation.
- [ ] For a managed event, mirror only lifecycle status; continue fetching native schedule/location when displaying.
- [ ] On native delete, mark/cancel extension, notify participants and waitlist once, then remove non-audit temporary extension state.
- [ ] Never recreate an externally deleted event.
- [ ] Ignore unrelated Scheduled Events and duplicate deliveries.
- [ ] On Ready/reconnect, perform one bounded reconciliation of nonterminal managed events against Discord to cover events missed while offline.

### Tests

- Update, completion, cancellation, deletion, duplicate delivery, unrelated event and restart reconciliation.

### Docs basis

Discord documents create/update/delete Gateway events and native [status automation](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-status-update-automation).

---

## #23 — Configure one Game Role mapping, Game Channel, Primary Voice Channel and Voice Pool

**Priority:** P0  
**Labels:** `type:feature`, `area:games`, `area:configuration`

**Blocked by:** #1

### Outcome

Each Guild can configure one initial game mapping; additional mappings remain deferred.

### Acceptance criteria

- [ ] `/game-config set` stores one Guild role, one game key/display name, one text Game Channel, one Primary Voice Channel and one or more unique voice channels in the Voice Pool.
- [ ] Store optional Discord Activity application ID plus required exact fallback activity name.
- [ ] Role and every channel must belong to the current Guild; Primary Voice Channel and Voice Pool entries must be voice channels.
- [ ] Primary Voice Channel must also be a member of the Voice Pool.
- [ ] Bot must effectively view the Game Channel and every Voice Pool channel.
- [ ] Enabling requires a configured Guild time zone; clear removes the mapping and closes no unrelated native event.
- [ ] `/settings` shows the complete mapping and disabled reason.

### Tests

- Wrong Guild/type, duplicate voice channel, missing time zone, permissions and round-trip persistence.

---

## #24 — Open one hostless Ad-hoc Game Session from a configured role mention

**Priority:** P0  
**Labels:** `type:feature`, `area:games`, `area:discord`

**Blocked by:** #17, #23

### Acceptance criteria

- [ ] Process a message only in the configured Game Channel and only when Discord's parsed role mentions include the exact configured role ID.
- [ ] Ignore bot/webhook messages and textual lookalikes that are not a parsed role mention.
- [ ] Create at most one nonterminal ad-hoc session for the mapping and one native `VOICE` Scheduled Event whose `channel_id` is the configured Primary Voice Channel.
- [ ] Attendance may still qualify from any configured channel in the Voice Pool; the Primary Voice Channel is only the native Scheduled Event location.
- [ ] Session has no Host; mention author gains no management privilege or Session Credit.
- [ ] Later qualifying mentions return/reuse the same session and never extend expiry.
- [ ] Concurrent qualifying mentions cannot create two sessions/events.
- [ ] The feature is an opt-in automation and therefore disabled until Game configuration is enabled.

### Tests

- Correct/wrong channel, exact role ID, bot message, concurrent mentions, reuse and no-expiry-extension.

### Docs basis

Discord's Message object exposes parsed [role mentions](https://docs.discord.com/developers/resources/message#message-object-message-structure); do not parse raw `<@&...>` text manually.

---

## #25 — Expire and recover Ad-hoc Game Sessions at the next local 05:00

**Priority:** P0  
**Labels:** `type:feature`, `area:games`, `area:time`

**Blocked by:** #24

### Acceptance criteria

- [ ] Persist the UTC expiry instant calculated from the Guild IANA time zone when session creation succeeds.
- [ ] At the boundary, cancel/delete the bot-created native event as appropriate and remove temporary ad-hoc state without another user notification.
- [ ] Startup/reconnect immediately expires overdue sessions and schedules only the next known deadline; do not add a generic job framework.
- [ ] Time-zone changes do not retroactively extend an already-created session.
- [ ] Ambiguous/nonexistent DST local times are resolved deterministically and covered by #1's time helper tests.
- [ ] Repeated expiry execution is idempotent.

### Tests

- Before/after 05:00, restart overdue, repeated expiry, time-zone change and DST boundary.

---

# v1.3 — Activity and Word Puzzle

## #26 — Enable Guild Presence observation and disclose its availability

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`, `area:discord`

**Blocked by:** #23

### Acceptance criteria

- [ ] Add a deployment setting controlling whether privileged `GUILD_PRESENCES` is requested and document the required Developer Portal toggle/review.
- [ ] Route Serenity `FullEvent::PresenceUpdate` without logging full activity payloads.
- [ ] Expose `available|degraded` Activity Detection status in `/settings`.
- [ ] When the setting is off, startup succeeds in degraded mode and manual attendance remains available; never request an intent known to be unavailable because Discord closes the Gateway with code `4014`.
- [ ] Do not infer that absence of a visible activity means the member is not playing; Discord lets users hide Activity Sharing.

### Tests

- Event routing and available/degraded status rendering.

### Docs basis

`GUILD_PRESENCES` is privileged in [Gateway intents](https://docs.discord.com/developers/events/gateway#privileged-intents). Users may hide activity globally, per Guild or per game according to Discord's [Activity Sharing FAQ](https://support.discord.com/hc/en-us/articles/7931156448919-Activity-Sharing-on-Discord-FAQ).

---

## #27 — Maintain an automatic Activity Beacon per matching voice channel

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`

**Blocked by:** #26

### Acceptance criteria

- [ ] A matching non-bot member in a configured Voice Pool channel starts/maintains that channel's beacon.
- [ ] Match a `Playing` activity by configured `application_id` first; when absent/not equal, use an exact case-insensitive activity-name comparison whose casing behavior is defined by focused tests.
- [ ] Do not fuzzy-match aliases or other activity types.
- [ ] Presence change, voice move/leave and member removal recompute only affected channels.
- [ ] Beacon never propagates to another voice channel in the pool.
- [ ] All non-bot members currently in a beaconed channel become eligible for attendance timing, even if only one member exposes the matching activity.

### Tests

- ID precedence, name fallback, wrong type/name, channel isolation, move and last matching member leaves.

### Docs basis

Discord's [Activity object](https://docs.discord.com/developers/events/gateway-events#activity-object) includes required name/type and optional `application_id`.

---

## #28 — Add manual voice check-in beacon fallback

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`, `area:interactions`

**Blocked by:** #27

### Acceptance criteria

- [ ] `/activity check-in` requires the caller to be a non-bot member currently in a configured session voice channel.
- [ ] Check-in is bound to member, session and current voice channel; moving/leaving clears that member's check-in.
- [ ] Manual beacon remains while at least one valid checked-in member remains in that channel.
- [ ] Multiple check-ins are idempotent; an automatic beacon and manual beacon may coexist without double timing.
- [ ] Manual fallback works when Presence status is degraded or the member hides activity.
- [ ] No moderator can check another member in during v1.3.

### Tests

- Not in voice, wrong channel/session, repeated check-in, mover/leaver and automatic/manual overlap.

---

## #29 — Persist resumable per-member attendance accumulation

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`, `area:database`

**Blocked by:** #28

### Outcome

Valid voice time can pause/resume without double counting, and a restart never counts bot downtime.

### Acceptance criteria

- [ ] Accumulate independently per `(guild, session, member)` only while member is in that session's beaconed voice channel.
- [ ] Persist accumulated whole seconds/minutes plus nullable active-start timestamp at bounded state transitions, not every tick.
- [ ] On graceful event transition, add elapsed time once and clear active start.
- [ ] On process startup, clear stale active starts without adding offline elapsed time; preserve prior accumulated duration.
- [ ] Bots and opted-out members are ignored.
- [ ] Reconnect within one session resumes from preserved accumulated duration.

### Tests

- Join/leave, beacon off/on, voice move, duplicate event, restart and individual member independence with a controlled clock.

---

## #30 — Qualify attendance at 30 cumulative minutes and finalize all valid time

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`

**Blocked by:** #29

### Acceptance criteria

- [ ] Less than 30:00 cumulative valid time grants no Play Time and no Session Credit.
- [ ] At 30:00 or more, finalization counts all accumulated valid time from zero, not only time after the threshold.
- [ ] Each member receives at most one Session Credit for a qualifying session.
- [ ] Finalization is transactional/idempotent by a stable attendance/session key.
- [ ] Integer rounding is defined once: sum elapsed seconds first, convert to whole display/storage minutes only at finalization; partial final minute is discarded.
- [ ] Session cancellation/deletion finalizes only members who already qualify.

### Tests

- 29:59, 30:00, interrupted accumulation, fractional final minute, repeated finalization and canceled session.

---

## #31 — Resolve Scheduled-versus-Ad-hoc overlap without double Play Time

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`, `area:games`

**Blocked by:** #30

### Acceptance criteria

- [ ] Detect overlap only for the same Guild, member and configured game.
- [ ] Each wall-clock interval contributes to Play Time once.
- [ ] When both session types cover the same interval, attribute it to the Scheduled Session.
- [ ] If both otherwise qualify, Scheduled Session gets the Session Credit and overlapping Ad-hoc Session does not.
- [ ] Non-overlapping portions remain eligible under their own session.
- [ ] Result is identical regardless of which session finalizes first.

### Tests

- Full overlap, partial overlap, different games, different members and reversed finalization order.

---

## #32 — Store aggregate Play Time and Session Credit and derive Activity Level

**Priority:** P0  
**Labels:** `type:feature`, `area:activity`, `area:database`

**Blocked by:** #31

### Data contract

Store per-Guild/per-member totals plus per-game totals. Store integer Play Time minutes and Session Credit separately. Do not store a mutable level or raw voice join/leave history after finalization.

### Acceptance criteria

- [ ] Finalized increments are atomic and deduplicated by source completion key.
- [ ] Total equals sum of per-game aggregates after every write.
- [ ] Level `n` is the greatest integer satisfying `n(n+1)/2` hours ≤ total Play Time.
- [ ] Level uses total minutes without floating-point comparison errors.
- [ ] Word Puzzle can add Session Credit without adding Play Time.
- [ ] Finalized raw transition rows are removed/compacted after aggregate commit.

### Tests

- Level 0/1/2 boundaries and one minute around each; idempotent increment; per-game sum; separate credits.

### Architecture reference

`docs/adr/0012-store-aggregate-activity-data-only.md`.

---

## #33 — Add Guild-scoped Activity Profile details

**Priority:** P1  
**Labels:** `type:feature`, `area:activity`, `area:commands`

**Blocked by:** #32

### Acceptance criteria

- [ ] `/activity profile [member]` shows total Play Time, Session Credit, derived Level, progress to next Level and per-game aggregates.
- [ ] Default target is caller; another member is viewable only within the current Guild and only when not opted out.
- [ ] Never display raw join/leave times or cross-Guild totals.
- [ ] Use Discord timestamps/duration formatting consistently and bound embed fields/pages.
- [ ] EN, VI and JA output exists.

### Tests

- Self/other member, cross-Guild isolation, opted-out target, level progress and large per-game list pagination.

---

## #34 — Add Play-Time leaderboard with shared ranks

**Priority:** P1  
**Labels:** `type:feature`, `area:activity`, `area:commands`

**Blocked by:** #32

### Acceptance criteria

- [ ] `/activity leaderboard` orders descending by total Play Time; deterministic secondary order affects display only, not rank.
- [ ] Equal minute totals share the same competition rank; the next rank skips accordingly (`1, 1, 3`).
- [ ] Display top 10 plus caller's row/rank when caller is outside top 10.
- [ ] Session Credit never breaks Play-Time ties.
- [ ] Exclude opted-out users and bot accounts.
- [ ] Query is scoped to one Guild and bounded.

### Tests

- Tie ranking, requester inside/outside top 10, zero rows, bot/opt-out exclusion and Guild isolation.

---

## #35 — Implement Activity tracking opt-out and clean re-entry

**Priority:** P0  
**Labels:** `type:feature`, `area:privacy`, `area:activity`

**Blocked by:** #32

### Acceptance criteria

- [ ] `/activity opt-out` requires explicit confirmation, stops new attendance immediately and deletes that member's Guild activity aggregates, active attendance, deduplication keys and bot-managed reward grants.
- [ ] It does not affect the member's data in another Guild or their global Tracker link.
- [ ] Profiles/leaderboards reveal no prior totals after completion.
- [ ] `/activity opt-in` re-enables tracking with empty totals; deleted data is not restored.
- [ ] Repeated opt-out/opt-in operations are idempotent.

### Tests

- Active-session opt-out, Guild isolation, deletion coverage, repeated calls and empty re-entry.

---

## #36 — Validate Activity Reward Role hierarchy and authority safety

**Priority:** P1  
**Labels:** `type:feature`, `area:permissions`, `area:activity`

**Blocked by:** #32

### Outcome

One shared validator prevents activity rewards from becoming an administrative path.

### Acceptance criteria

- [ ] Reject `@everyone`, managed/integration roles, roles at or above the bot's highest role, and roles the bot cannot assign.
- [ ] Reject a role whose base permissions include `ADMINISTRATOR` or any moderation/management permission enumerated and documented in the implementation.
- [ ] Inspect channel permission overwrites for that role and reject explicit grants of the same unsafe permissions.
- [ ] Return the exact unsafe permission/reason to the administrator.
- [ ] Reuse Discord/Serenity effective role hierarchy data; do not invent a custom hierarchy.

### Tests

- Safe role, administrator, moderation bit, channel overwrite escalation, managed role and hierarchy boundary.

### Docs basis

Discord documents [role hierarchy](https://docs.discord.com/developers/topics/permissions#role-hierarchy) and channel overwrite calculation in the [permissions reference](https://docs.discord.com/developers/topics/permissions#permission-overwrites).

---

## #37 — Configure, create and safely replace one Activity Reward Role

**Priority:** P1  
**Labels:** `type:feature`, `area:activity`, `area:configuration`

**Blocked by:** #35, #36

### Acceptance criteria

- [ ] `/activity reward set <level> [role]` selects a safe existing role or creates one safe role when omitted; level must be positive.
- [ ] Persist role ID, threshold and ownership (`guild_owned|bot_owned`).
- [ ] Track each grant made by the bot separately from Discord's current member-role list.
- [ ] On role/threshold change, reconcile all bot-managed grants: add eligible, remove only bot-tracked grants from ineligible users, and migrate eligible users to the new role.
- [ ] Replace: delete the old role only when recorded `bot_owned`; for `guild_owned`, remove bot-tracked grants and stop using it without deleting it.
- [ ] Never remove a manual grant that is not in bot grant tracking.
- [ ] If later role update/channel overwrite makes the configured role unsafe, stop new grants, mark configuration degraded and notify the administrator/Moderation Channel.

### Tests

- Auto-create, existing role, threshold up/down, role replacement for both ownership types, manual grant preservation and later permission escalation.

### Architecture reference

`docs/adr/0010-protect-guild-owned-activity-reward-roles.md`.

---

## #38 — Select and check in a licensed curated English five-letter Word Set

**Priority:** P1  
**Labels:** `type:content`, `area:minigame`, `area:legal`

### Outcome

The repository contains a reproducible answer list and allowed-guess list with a verified redistribution license.

### Acceptance criteria

- [ ] Record source URL, exact revision/date and license text/attribution in the repository before importing words.
- [ ] Do not copy the proprietary Wordle answer list without a compatible redistribution basis.
- [ ] Every entry is normalized lowercase five-letter ASCII; duplicates and malformed entries fail a repository test.
- [ ] Every answer is also in allowed guesses.
- [ ] Review/remove offensive answers from the smaller answer list while retaining a documented neutral policy.
- [ ] Words remain English; only surrounding UI is localized.

### Tests

- One fast data-integrity test validates format, uniqueness, subset relationship and nonempty lists.

### Note

The exact third-party list is intentionally not named until its license and revision are verified; selecting it is the work of this issue, not an assumption in the backlog.

---

## #39 — Implement and test the pure Word Puzzle rules engine

**Priority:** P1  
**Labels:** `type:feature`, `area:minigame`

**Blocked by:** #38

### Acceptance criteria

- [ ] Engine accepts one hidden five-letter answer and up to six valid guesses.
- [ ] Reject malformed/not-allowed guesses without consuming an attempt.
- [ ] Produce correct exact/present/absent feedback with duplicate letters using a two-pass count algorithm or an equivalently proven method.
- [ ] Stop accepting guesses after win or sixth valid guess.
- [ ] Engine contains no Discord, database, clock or random-number access.
- [ ] Given answer and guesses, result is deterministic.

### Tests

- Exact match, absent letters, repeated letters in answer, excess repeated letters in guess, invalid word, win and six-guess loss.

---

## #40 — Persist competitive Word Puzzle Session and private participant boards

**Priority:** P1  
**Labels:** `type:feature`, `area:minigame`, `area:database`

**Blocked by:** #39

### Acceptance criteria

- [ ] Create one Guild-scoped puzzle session with creator, answer reference/value, created/start/deadline state and stable random selection.
- [ ] Join is allowed before start; start freezes roster and gives every participant the same answer.
- [ ] Store each participant's guesses/status privately; another participant cannot query unfinished guesses.
- [ ] Restart preserves answer, deadline and boards; it never rerolls.
- [ ] Expired sessions finish through a bounded startup/read/write reconciliation, not a generic scheduler framework.
- [ ] Retention removes detailed boards after the final result has been delivered while keeping only the completion key required for daily credit deduplication.

### Tests

- Two participants same answer, authorization/privacy, restart, deadline and cleanup.

---

## #41 — Add Word Puzzle commands/interactions and delayed result summary

**Priority:** P1  
**Labels:** `type:feature`, `area:minigame`, `area:interactions`

**Blocked by:** #40

### Acceptance criteria

- [ ] Provide create, join, start, private guess, private status and finish interactions as slash commands/buttons/modals supported by Poise/Serenity.
- [ ] Guess and board responses are ephemeral/private.
- [ ] Do not reveal the answer or another board until every participant finishes or the deadline expires.
- [ ] Final summary shows each participant's solved/unsolved status and attempt count; it does not expose unfinished guesses early.
- [ ] Duplicate interaction delivery is idempotent.
- [ ] All surrounding UI and errors exist in EN, VI and JA.

### Tests

- Early secrecy, all-finished reveal, deadline reveal, duplicate submission and localization-key completeness.

---

## #42 — Award at most one Word Puzzle Session Credit per Guild-day

**Priority:** P2  
**Labels:** `type:feature`, `area:minigame`, `area:activity`

**Blocked by:** #1, #32, #41

### Acceptance criteria

- [ ] A participant earns one Session Credit after valid puzzle completion, regardless of win, speed or guess count.
- [ ] No Play Time is added.
- [ ] Deduplicate by `(guild_id, member_id, puzzle-credit Guild-day)` where the Guild-day changes at local 00:00 in the configured IANA time zone.
- [ ] Two puzzles in one Guild-day award one total credit; another Guild has an independent allowance.
- [ ] Finalization/retry/restart cannot double-credit.
- [ ] An opted-out member receives no credit.

### Tests

- Win/loss equality, same-day duplicate, 04:59/05:00 boundary, separate Guilds, retry and opt-out.

---

# v1.4 — VALORANT links

## #43 — Validate and persist one global Tracker Profile Link per Discord user

**Priority:** P1  
**Labels:** `type:feature`, `area:valorant`, `area:database`

### Outcome

A Discord user can save one navigation link shared across every Guild; it is not a Riot identity proof.

### Acceptance criteria

- [ ] Store exactly one row keyed by Discord user ID, with normalized HTTPS URL and timestamps; no `guild_id` column.
- [ ] Accept only the documented Tracker Network VALORANT profile host/path shape verified during implementation.
- [ ] Reject HTTP, user-info credentials, non-default ports, fragments, unrelated/subdomain-confusion hosts and non-profile paths.
- [ ] Preserve only navigation data; never fetch the submitted URL, resolve stats or infer Riot PUUID/account ownership.
- [ ] Updating replaces the user's prior global link; removing deletes it globally.
- [ ] Guild-removal cleanup does not delete this table.

### Tests

- URL parser table covering valid encoded Riot names/tags and malicious host/path variants; global replacement and Guild cleanup preservation.

### Policy basis

Tracker Network states that it does not offer a public VALORANT API for this use and blocks scraping in its [developer response](https://feedback.tracker.gg/t/development-of-a-bot-discord-stats-valorant/43663). Automated player data remains deferred to Riot-approved RSO.

---

## #44 — Add self-managed Tracker link commands and public Guild viewing

**Priority:** P1  
**Labels:** `type:feature`, `area:valorant`, `area:commands`

**Blocked by:** #43

### Visibility contract

The link is user-supplied, global and viewable by members who share the command Guild with its owner. It must always be labeled external/unverified; no per-Guild link value or visibility setting exists in v1.4.

### Acceptance criteria

- [ ] `/valorant tracker set <url>` and `remove` can modify only the caller's link.
- [ ] `/valorant tracker view [member]` supports self-view and viewing another current member of the command Guild; it never offers arbitrary global user lookup.
- [ ] Viewing from different shared Guilds returns the same global URL.
- [ ] Output clearly says `User-provided external link — not verified by Riot or this bot` in EN, VI and JA.
- [ ] Discord embed/button opens the URL but the bot makes no HTTP request to Tracker.
- [ ] Missing link and removed link return localized neutral responses.

### Tests

- Self-update authorization, same link across two Guilds, nonmember lookup rejection, removal and unverified label.

### Out of scope

Rank, match history, Riot account linking, link ownership verification, scraping and per-Guild visibility.

---

# v2.0 — Slash-only

## #45 — Inventory prefix-only behavior and publish the breaking-change map

**Priority:** P1  
**Labels:** `type:docs`, `area:commands`, `breaking-change`

### Outcome

Every current prefix surface has an identified slash replacement before removal.

### Acceptance criteria

- [ ] Enumerate every `prefix_command`, dynamic prefix lookup and prefix-only parsing path with its slash replacement.
- [ ] Record commands already slash-only (`/messagelog`, `/language`) so they are not accidentally changed.
- [ ] Identify documentation/translations/database columns that exist only for prefix support.
- [ ] Publish an upgrade note with the removal version and command mapping.
- [ ] Do not add a second deprecation framework or telemetry system.

### Tests

- A repository search accounts for every `prefix_command`, prefix callback/configuration reference and prefix-only translation in the published map.

### Current code anchors

`src/app.rs`, `src/commands/configuration/prefix.rs`, `src/commands/configuration/settings.rs`, all `#[poise::command(... prefix_command ...)]` occurrences.

---

## #46 — Remove prefix command registration and per-Guild prefix configuration

**Priority:** P0  
**Labels:** `type:feature`, `area:commands`, `breaking-change`

**Blocked by:** #45

### Acceptance criteria

- [ ] Remove every `prefix_command` flag and the dynamic prefix callback from Poise configuration.
- [ ] Remove prefix configuration commands/module, prefix settings output, obsolete translations and runtime database reads.
- [ ] Add a forward migration that safely drops or stops using prefix storage according to SQLite capabilities; existing Guild settings remain intact.
- [ ] Preserve slash command names, permissions, cooldowns, autocomplete, buttons and modals.
- [ ] Owner presence management remains available through slash commands.
- [ ] Text that resembles the old prefix commands produces no bot command response.

### Tests

- Command registration snapshot/list, representative slash moderation/config/general commands and no prefix dispatch.

---

## #47 — Retain Message Content only for opt-in Message Log and complete release verification

**Priority:** P0  
**Labels:** `type:maintenance`, `area:discord`, `area:docs`

**Blocked by:** #14, #46

### Acceptance criteria

- [ ] Keep `MESSAGE_CONTENT` intent solely because opt-in edited/deleted Message Log needs content when Discord grants it.
- [ ] No command parser depends on Message Content.
- [ ] Degraded Message Log remains explicit and metadata-safe when content access is denied.
- [ ] Update README variants and deployment/Developer Portal instructions for slash-only behavior and remaining privileged intents (`MESSAGE_CONTENT`, `GUILD_PRESENCES`, plus nonprivileged intents used by features).
- [ ] Run `cargo fmt --check`, strict Clippy, `cargo test --locked` and a release build successfully.
- [ ] Document the v2.0 breaking change and rollback constraint; do not maintain a hidden compatibility prefix.

### Tests

- Verify startup with Message Content deployment setting both enabled and disabled; run the full format, Clippy, test and release-build commands listed above.

### Docs basis

Discord's [Message Content intent](https://docs.discord.com/developers/events/gateway#message-content-intent) controls message content fields independently of application commands.

---

# Explicitly deferred — do not create milestone issues yet

- Multiple Game Role mappings, aliases or automatic game discovery.
- `EXTERNAL` or `STAGE_INSTANCE` Community Activities.
- Generic mini-game engine, custom Word Sets, custom words, inventories or economies.
- Tracker scraping or unofficial VALORANT stats.
- Riot RSO until production access is approved.
- PostgreSQL, sharding, distributed locks, multiple bot editions and a general-purpose web dashboard.
