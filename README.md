# 🤖 Rust Discord Bot

High-performance, modular Discord bot built with **Rust**, **Poise** (on **Serenity**), and **SQLite**.

## Architecture

```
src/
├── main.rs              # Entry point — framework setup, logging, client
├── lib.rs               # Shared types: Data, Context, Error
├── config.rs            # Environment variable loading
├── database.rs          # SQLite connection pool + migrations
├── error.rs             # Centralized error handler
└── commands/
    ├── mod.rs           # Command registry (add new commands here)
    ├── general.rs       # /ping, /botinfo, /serverinfo
    ├── moderation.rs    # /kick, /ban, /purge
    ├── administration.rs # /settings, /setprefix
    └── music.rs         # /play, /stop (placeholder)
```

## Quick Start

### 1. Prerequisites
- [Rust](https://rustup.rs/) (stable)
- A Discord bot token from [Discord Developer Portal](https://discord.com/developers/applications)

### 2. Setup
```bash
cp .env.example .env
# Edit .env and set your DISCORD_TOKEN
```

### 3. Run (Development)
```bash
cargo run
```

### 4. Build (Release)
```bash
cargo build --release
# Binary: target/release/my_rust_bot
```

## Adding New Commands

1. Create `src/commands/your_module.rs`
2. Add your command functions with `#[poise::command(slash_command, prefix_command)]`
3. Register in `src/commands/mod.rs`:
   ```rust
   pub mod your_module;
   // In all() function, add:
   your_module::your_command(),
   ```

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

| Component | Technology |
|-----------|-----------|
| Language | Rust 🦀 |
| Framework | Poise + Serenity |
| Async Runtime | Tokio |
| Database | SQLite (via SQLx) |
| Logging | tracing + tracing-subscriber |
| Process Manager | Systemd |
