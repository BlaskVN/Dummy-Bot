# Dummy Bot

[Tiếng Việt](README.vi.md)

Dummy Bot is a Discord bot built with Rust, Poise/Serenity, and SQLite. Runtime
settings live in `config.env`, secrets in `.env`, and deployment settings in
`.deploy.env`.

## Features

- General server and bot information commands
- Kick, ban, and purge moderation commands with Discord permission checks
- Per-guild prefix, language, and message-log configuration
- Deleted/edited message logging with bounded attachment archiving
- Persistent bot presence and voice-channel reconnect support

## Requirements and local setup

Install the stable Rust toolchain and create a bot in the Discord Developer
Portal. Enable the Message Content and Server Members privileged intents; voice
commands also require the Voice States intent.

```bash
cp .env.example .env
# Set DISCORD_TOKEN in .env. Non-secret defaults are in config.env.
cargo run
```

Run the checks expected before merging:

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
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
│   ├── configuration/      # language, prefix, logging, settings
│   ├── presence.rs
│   └── voice.rs
├── handlers/               # event dispatch and event features
└── i18n.rs                 # user-facing EN/VI strings
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
| Ownership/defaults | `OWNER_IDS`, `DEFAULT_PREFIX`, `DEFAULT_LANGUAGE` |
| Command limits | `PREFIX_MAX_CHARS`, `PURGE_MAX_MESSAGES`, `PURGE_CONFIRMATION_SECONDS`, `BAN_MAX_DELETE_DAYS`, `PRESENCE_MAX_DURATION_MINUTES` |
| Runtime/recovery | `CACHE_MAX_MESSAGES`, `GATEWAY_RESUME_DELAY_SECONDS`, `GATEWAY_READY_DELAY_SECONDS` |
| Message logging | `MESSAGE_PREVIEW_CHARS`, `MESSAGE_LOG_CHUNK_CHARS`, `MESSAGE_TIMESTAMP_FORMAT`, `ATTACHMENT_MAX_BYTES`, `PURGE_ATTACHMENT_MAX_TOTAL_BYTES` |
| Appearance | all `EMBED_COLOR_*` variables |

`OWNER_IDS` is a comma-separated list of Discord user IDs. Leave it empty to
use the Discord application or team owner. Attachment limits default to 10 MiB
per file and 64 MiB per purge; raise them only when the destination guild and
host resources support the larger transfer.

Per-guild prefixes are stored in SQLite. `DEFAULT_PREFIX` is only used until a
guild saves its own prefix.

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

V1 uses only `migrations/0001_initial.sql`; no additional migration is needed.

## Discord permissions

Privileged commands combine Poise checks with Discord's command permission
metadata. Kick and ban also verify the caller's and bot's role hierarchy.
Voice and message logging evaluate permissions in the destination channel,
including channel overwrites.

| Command | User permission | Bot permission |
|---|---|---|
| `/kick` | `KICK_MEMBERS` and a role above the target | `KICK_MEMBERS` and a role above the target |
| `/ban` | `BAN_MEMBERS` and a role above the target | `BAN_MEMBERS` and a role above the target |
| `/purge` | `MANAGE_MESSAGES` | `VIEW_CHANNEL`, `MANAGE_MESSAGES`, `READ_MESSAGE_HISTORY` |
| `/settings`, `/setprefix`, `/messagelog`, `/language` | `MANAGE_GUILD` | depends on the requested action/channel |
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
