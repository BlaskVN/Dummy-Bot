# Dummy Bot Roadmap

This roadmap grows the existing `v1.0.0` bot into a publicly installable, multi-Guild community bot. Versions are ordered milestones, not calendar commitments.

## Rules for every release

- Use native Discord permissions; Bot Owner authority stays separate from Guild administration.
- Interactive Commands are available by default; Automations require per-Guild opt-in.
- Ship every user-facing feature in English, Vietnamese, and Japanese.
- Isolate Guild Data and delete it when the bot is permanently removed from a Guild.
- Continue safely with an explicit degraded status when a privileged Discord intent is unavailable.
- Keep one process and SQLite until measured adoption or load requires another architecture.

## v1.1 — Public foundation and moderation

- Send one Onboarding Message by DM when the installer can be identified without extra permissions, otherwise use an available system channel.
- Add Guild Time Zone configuration using IANA names.
- Let the Bot Owner set or clear Donation Information, including a validated uploaded PNG/JPEG QR image; expose it only through `/donate` and `/botinfo`.
- Add a separate Moderation Channel.
- Add per-Guild numbered, immutable Moderation Cases for warn, kick, ban, and timeout; incorrect cases are voided with an audit reason.
- Store only case metadata and an optional Discord evidence link, never copied message content or attachments.
- Consume Discord AutoMod execution events without recreating AutoMod configuration. Notify moderators and open one review suggestion after the same member triggers the same rule three times within seven days; keep it deduplicated until a moderator handles it or Discord reports that exact rule was updated. Never punish a member or change a rule automatically.
- Delete Guild Data only on permanent removal, not temporary Discord unavailability.
- Expose Degraded Message Log status and send one channel warning when Message Content access is unavailable.

## v1.2 — Community Activities and Game Sessions

- Represent every scheduled Community Activity with a Discord Scheduled Event; v1.2 supports `VOICE` events only.
- Require native `CREATE_EVENTS`; let the Host manage their activity and `MANAGE_EVENTS` manage any activity.
- Treat Discord subscriptions as interest only. Joining through the bot reserves a place; overflow enters a FIFO waitlist and promotion is automatic.
- Cancel the bot extension when its Scheduled Event is deleted, notify Participants and Waitlisted Members once, and never recreate the event.
- Map an initial Game Role to one game, one Game Channel, one Primary Voice Channel used as the native Scheduled Event location, and a Voice Pool used for attendance.
- Mentioning the Game Role in the Game Channel opens one hostless Ad-hoc Game Session until the next 05:00 Guild Time Zone boundary. Later mentions reuse it without extending the deadline.

## v1.3 — Activity and Word Puzzle

- Request Guild Presences and establish an Activity Beacon per voice channel from a matching Discord application ID, then exact case-insensitive activity name.
- Allow a manual check-in beacon when Presence is unavailable or private; every member still needs 30 cumulative voice minutes.
- Qualifying sessions count all valid minutes from the start. Overlapping minutes count once, with Scheduled Sessions taking precedence over Ad-hoc Sessions.
- Track Session Credit separately from Play Time. Derive Activity Level from total hours using `n(n+1)/2` hours for Level `n`.
- Provide per-Guild Activity Profiles, per-game aggregate breakdowns, and a Play-Time leaderboard with shared ranks, top ten, and the requester's position.
- Support per-member opt-out that erases their activity aggregates.
- Configure one Activity Reward Role and threshold. The role may be bot-created or Guild-owned, must not carry moderation authority, and is reconciled without touching manual grants.
- Add competitive Word Puzzle Sessions using the built-in five-letter English Word Set, six private guesses, and delayed results. A valid completion grants at most one Session Credit per local calendar day (00:00 in Guild Time Zone) and no Play Time.

## v1.4 — VALORANT links

- Let each Discord user attach or remove one global Tracker Profile Link for navigation only; the link cannot vary by Guild and may be viewed by members who share a Guild with its owner.
- Never verify identity through tracker.gg, import tracker.gg data, or scrape it.

## Riot-approved milestone

Schedule this only after Riot grants production access:

- Deploy the minimal Companion Website with bot information, Privacy Policy, Terms, Riot Sign On, unlinking, and data-deletion requests.
- Link one Riot account globally per Discord user, while keeping Guild Profile Visibility opt-in and hidden by default.
- Show official rank, recent matches, and an opt-in Guild VALORANT Leaderboard.
- Never calculate alternative MMR/ELO or provide opponent scouting.

## v2.0 — Slash-only commands

- Remove legacy prefix commands as the deliberate breaking change.
- Retain optional Message Content access only for Message Log; preserve explicit degraded behavior if Discord denies it.

## v3.0 — Modular Architecture with Native Rust Core & Rhai Script Modules

- Refactor core system into a high-performance native Rust core (Discord Gateway, SQLx SQLite, State Management, Event Bus Dispatcher).
- Integrate Rhai v1.25.1 embedded scripting engine with `sync` and `serde` features.
- Move surrounding feature modules into `/modules/*.rhai` scripts (`automod.rhai`, `moderation.rhai`, `word_puzzle.rhai`, `attendance.rhai`, `community.rhai`).
- Add `/reload_modules` slash command for live hot-reloading without restarting the bot.

## Deferred until demonstrated need

- Additional Game Role mappings, Game Integrations, Mini-games, Word Sets, custom words, reward tiers, and reward perks.
- A web dashboard beyond account/legal flows.
- PostgreSQL, sharding, distributed coordination, or multiple bot editions.
