# Rust Discord Bot

High-performance, modular Discord bot built with **Rust**, **Poise** (on **Serenity**), and **SQLite**.

## Features

- **Moderation** — kick, ban, bulk purge with detailed logging
- **Message Logging** — tracks edits, deletes, and bulk deletes to a configured log channel
- **Voice Support** — join/leave voice channels via Songbird, with kick notifications
- **Multi-language (i18n)** — English, Vietnamese, Japanese
- **Custom Prefix** — per-guild configurable prefix
- **Owner Commands** — Rich Presence control (status, activity, clear)
- **Auto-Recovery** — Systemd service with crash restart and rate-limit protection

## Architecture

```
src/
├── main.rs                          # Entry point — framework setup, event handlers, client
├── lib.rs                           # Shared types: Data, Context, Error
├── config.rs                        # Environment variable loading
├── database.rs                      # SQLite connection pool + migrations
├── error.rs                         # Centralized error handler (localized)
├── i18n.rs                          # Internationalization (en, vi, ja)
├── handlers/
│   ├── mod.rs                       # Event handler dispatcher
│   ├── message_log.rs               # Message delete/edit/bulk-delete logging
│   └── voice.rs                     # Voice state change notifications
└── commands/
    ├── mod.rs                       # Command registry (guild + global)
    ├── guild/                       # Guild-only commands
    │   ├── everyone/
    │   │   ├── serverinfo.rs        # /serverinfo
    │   │   └── voice.rs             # /connect, /disconnect
    │   ├── moderator/
    │   │   ├── kick.rs              # /kick
    │   │   ├── ban.rs               # /ban
    │   │   └── purge.rs             # /purge
    │   └── admin/
    │       ├── settings.rs          # /settings
    │       ├── setprefix.rs         # /setprefix
    │       ├── logging.rs           # /messagelog enable|disable|status
    │       └── language.rs          # /language
    └── global/                      # Works in DMs and guilds
        ├── everyone/
        │   ├── ping.rs              # /ping
        │   └── botinfo.rs           # /botinfo
        └── owner/
            └── presence.rs          # /presence status|activity|clear
```

## Commands

| Command               | Permission      | Description                                                     |
|-----------------------|-----------------|-----------------------------------------------------------------|
| `/ping`               | Everyone        | Check bot latency                                               |
| `/botinfo`            | Everyone        | Show bot info and uptime                                        |
| `/serverinfo`         | Everyone        | Show server details (members, channels, roles)                  |
| `/connect`            | Everyone        | Join your voice channel                                         |
| `/disconnect`         | Everyone        | Leave the voice channel                                         |
| `/kick`               | Kick Members    | Kick a member with optional reason                              |
| `/ban`                | Ban Members     | Ban a member with optional reason and message deletion days     |
| `/purge`              | Manage Messages | Bulk delete 1-100 messages                                      |
| `/settings`           | Manage Guild    | Show current guild configuration                                |
| `/setprefix`          | Manage Guild    | Set custom command prefix (1-5 chars)                           |
| `/messagelog enable`  | Manage Guild    | Enable message logging to a channel                             |
| `/messagelog disable` | Manage Guild    | Disable message logging                                         |
| `/messagelog status`  | Manage Guild    | Show logging configuration                                      |
| `/language`           | Manage Guild    | Set bot language (en, vi, ja)                                   |
| `/presence status`    | Owner           | Set bot online status (Online/Idle/DND/Invisible)               |
| `/presence activity`  | Owner           | Set Rich Presence (Playing/Listening/Watching/Competing/Custom) |
| `/presence clear`     | Owner           | Clear activity and reset to Online                              |

## Quick Start

### 1. Prerequisites

- [Rust](https://rustup.rs/) (stable)
- A Discord bot token from [Discord Developer Portal](https://discord.com/developers/applications)

### 2. Setup

```bash
cp .env.example .env
# Edit .env and set your DISCORD_TOKEN and OWNER_ID
```

### 3. Run (Development)

```bash
cargo run
```

### 4. Build (Release)

```bash
cargo build --release
# Binary: target/release/rust_discord_bot
```

## Configuration

| Variable        | Required | Default                       | Description                             |
|-----------------|----------|-------------------------------|-----------------------------------------|
| `DISCORD_TOKEN` | Yes      | —                             | Bot token from Discord Developer Portal |
| `DATABASE_URL`  | No       | `sqlite:data/bot.db?mode=rwc` | SQLite database path                    |
| `RUST_LOG`      | No       | `rust_discord_bot=info`       | Log level filter                        |
| `OWNER_ID`      | No       | Auto-detected                 | Bot owner's Discord user ID             |

## Deployment (Linux Server + Systemd)

### First-time Server Setup

```bash
# On the server:
sudo useradd -m -s /bin/bash bot_user
sudo mkdir -p /home/bot_user/bot/data
sudo chown -R bot_user:bot_user /home/bot_user/bot

# Copy the .env file
scp .env bot_user@your-server:/home/bot_user/bot/.env

# Install the systemd service
sudo cp systemd/discord-bot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable discord-bot  # Auto-start on boot
sudo systemctl start discord-bot
```

### Deploy Updates

```bash
./deploy.sh bot_user@your-server
```

### Monitor

```bash
# View logs
journalctl -u discord-bot -f

# Check status
systemctl status discord-bot

# Manual restart
sudo systemctl restart discord-bot
```

## Auto-Recovery

The Systemd service is configured with:

- **`Restart=always`** — restarts on any crash or exit
- **`RestartSec=10`** — 10-second delay to avoid API rate limits
- **`After=network-online.target`** — waits for network after power outage
- **Crash loop protection** — stops if 5+ restarts within 60 seconds

## Tech Stack

| Component       | Technology                   |
|-----------------|------------------------------|
| Language        | Rust                         |
| Framework       | Poise + Serenity             |
| Async Runtime   | Tokio                        |
| Database        | SQLite (via SQLx)            |
| Voice           | Songbird                     |
| HTTP Client     | Reqwest (rustls)             |
| Logging         | tracing + tracing-subscriber |
| i18n            | Custom (en, vi, ja)          |
| Process Manager | Systemd                      |
