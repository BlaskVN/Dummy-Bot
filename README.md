# Dummy Bot

[Tiếng Việt](README.vi.md)

Dummy Bot is a Discord bot built with Rust, Poise/Serenity, and SQLite. Runtime
settings live in `config.env`, secrets in `.env`, and deployment settings in
`.deploy.env`.

## Features

- General server and bot information commands
- Kick, ban, and purge moderation commands with Discord permission checks
- Slash-only commands with per-Guild language and Message Log configuration
- Deleted/edited message logging with bounded attachment archiving
- Persistent bot presence and voice-channel reconnect support
- Guild time zones and one-time installation onboarding
- Owner-managed donation information with stable QR storage
- Immutable moderation cases, dedicated moderation channels, and warn/timeout commands
- Opt-in Discord AutoMod observation with bounded review suggestions

## Requirements and local setup

Install the stable Rust toolchain and create a bot in the Discord Developer
Portal. Install it with the `bot` and `applications.commands` scopes. Enable the
Server Members privileged intent. Voice States, Guild Messages, Scheduled
Events, and AutoMod event intents are nonprivileged and requested for their
respective features.

Message Content is optional and is not used by command parsing. Enable it in the
Developer Portal and set `MESSAGE_CONTENT_ENABLED=true` only when full
edited/deleted Message Log content is required. Otherwise set it to `false`;
the bot starts normally, keeps metadata-safe logging, and explicitly marks an
enabled Message Log as degraded.

Set `GUILD_PRESENCES_ENABLED=true` only after enabling Presence Intent in the
Developer Portal (and completing Discord review when required). Leave it false
to start in degraded Activity Detection mode; manual attendance still works.

```bash
cp .env.example .env
# Set DISCORD_TOKEN in .env. Non-secret defaults are in config.env.
cargo run
```

Run the checks expected before merging:

```bash
cargo test --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked
```

## Project layout

```text
src/
├── main.rs                 # environment and tracing setup
├── app.rs                  # Discord framework/client initialization
├── state.rs                # shared feature state
├── config.rs               # runtime configuration parsing and validation
├── permissions.rs          # permissions, overwrites, and role hierarchy
├── database.rs             # pool initialization and shared queries
├── commands/
│   ├── general/            # ping, botinfo, serverinfo
│   ├── moderation/         # kick, ban, purge
│   ├── configuration/      # language, logging, settings
│   ├── presence.rs
│   └── voice.rs
├── handlers/               # event dispatch and event features
└── i18n.rs                 # user-facing EN/VI/JA strings
migrations/                 # embedded SQLx migrations
systemd/                    # deployment service template
```

Add a command or handler module to the appropriate directory, then register it
in the nearest `all()` or `dispatch()` function. Authorization is based on
Discord permission flags, not role names.

## Runtime configuration

`Config::load()` reads the non-secret configuration from `config.env` and the
bot token from `.env`. Missing or invalid values stop startup with an explicit
error.

| Group | Variables |
|---|---|
| Secret (`.env`) | `DISCORD_TOKEN` |
| Connection | `DATABASE_URL`, `DATA_DIRECTORY`, `RUST_LOG` |
| Ownership/defaults | `OWNER_IDS`, `DEFAULT_LANGUAGE` |
| Command limits | `PURGE_MAX_MESSAGES`, `PURGE_CONFIRMATION_SECONDS`, `BAN_MAX_DELETE_DAYS`, `PRESENCE_MAX_DURATION_MINUTES` |
| Runtime/recovery | `CACHE_MAX_MESSAGES`, `GATEWAY_RESUME_DELAY_SECONDS`, `GATEWAY_READY_DELAY_SECONDS`, `GUILD_PRESENCES_ENABLED` |
| Message logging | `MESSAGE_CONTENT_ENABLED`, `MESSAGE_PREVIEW_CHARS`, `MESSAGE_LOG_CHUNK_CHARS`, `MESSAGE_TIMESTAMP_FORMAT`, `ATTACHMENT_MAX_BYTES`, `PURGE_ATTACHMENT_MAX_TOTAL_BYTES` |
| Appearance | all `EMBED_COLOR_*` variables |

`OWNER_IDS` is a comma-separated list of Discord user IDs. Leave it empty to
use the Discord application or team owner. Attachment limits default to 10 MiB
per file and 64 MiB per purge; raise them only when the destination guild and
host resources support the larger transfer.

Commands are slash-only. v2.0 removes per-Guild prefixes and does not retain a
hidden text-command compatibility mode. See
[the v2.0 upgrade note](docs/v2-slash-only-upgrade.md) before upgrading; the
forward migration discards saved prefix values, so database rollback requires a
pre-upgrade backup.

## Database migrations

Migrations are required because guild configuration and persistent presence are
stored in SQLite. They are embedded in the binary and applied by
`sqlx::migrate!()` during startup.

- Never edit a migration that may have run in an existing environment; SQLx
  verifies its checksum.
- Add a new numbered SQL file only when the schema changes.
- Do not add an empty migration for an application-only release.
- Verify a schema change with `cargo test database::tests::applies_initial_migration`.
- Back up the SQLite database before deploying a release with a destructive
  schema change.

Current releases apply numbered migrations in order; `0001_initial.sql` remains
the upgrade baseline and must not be edited.

## Discord permissions

Privileged commands combine Poise checks with Discord's command permission
metadata. Kick and ban also verify the caller's and bot's role hierarchy.
Voice and message logging evaluate permissions in the destination channel,
including channel overwrites. Message Log shows the original author in the
embed body and thumbnail; it does not infer the deleting actor from audit logs.

| Command | User permission | Bot permission |
|---|---|---|
| `/kick` | `KICK_MEMBERS` and a role above the target | `KICK_MEMBERS` and a role above the target |
| `/ban` | `BAN_MEMBERS` and a role above the target | `BAN_MEMBERS` and a role above the target |
| `/purge` | `MANAGE_MESSAGES` | `VIEW_CHANNEL`, `MANAGE_MESSAGES`, `READ_MESSAGE_HISTORY` |
| `/warn`, `/timeout`, `/case view`, `/case list` | `MODERATE_MEMBERS` and target hierarchy | `MODERATE_MEMBERS` and target hierarchy for timeout |
| `/settings`, `/messagelog`, `/language`, `/timezone`, `/moderation-channel`, `/automod-observer`, `/case void` | `MANAGE_GUILD` | depends on the requested action/channel |
| `/donation` | Bot Owner | — |
| `/connect` | `MOVE_MEMBERS`, while in a voice channel with `CONNECT` | `VIEW_CHANNEL`, `CONNECT` in that channel |
| `/disconnect` | `MOVE_MEMBERS` | no member-management permission is needed to leave |
| `/presence` | an owner from `OWNER_IDS` or the Discord application owner | — |

## Release and deployment

For a release, update the package version in `Cargo.toml`, run the checks above,
commit to `main`, and create an annotated `vX.Y.Z` tag on that commit. `Cargo.lock`
must remain committed.

Deployment builds locally, atomically replaces the remote binary, and updates
`config.env`:

```bash
cp .deploy.env.example .deploy.env
# Set every DEPLOY_* value.
./deploy.sh
```

Before restarting the service, confirm the Developer Portal intent toggles
match `config.env`: Server Members is required; Presence must match
`GUILD_PRESENCES_ENABLED`; Message Content must match
`MESSAGE_CONTENT_ENABLED`. Verified applications may require Discord approval
for privileged intents. Slash commands do not require Message Content.

The first installation also needs a systemd unit. Load `.deploy.env`, render
`systemd/discord-bot.service.template` by replacing each `@...@` placeholder,
install it under `/etc/systemd/system`, create `DEPLOY_DATA_DIRECTORY`, then run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now "$DEPLOY_SERVICE_NAME"
journalctl -u "$DEPLOY_SERVICE_NAME" -f
```

Keep the remote `.env` root-owned with mode `0600`. `deploy.sh` deliberately
does not upload secrets.

## Dependency maintenance

`Cargo.toml` is the source of truth for direct dependencies and `Cargo.lock`
pins the complete build. Review release notes before upgrading Poise/Serenity,
SQLx, Rustls, or Tokio, then rerun the full check sequence. The bot sends voice
state directly through Serenity and does not use an audio driver.
