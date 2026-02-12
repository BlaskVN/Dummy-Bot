#!/bin/bash
# =============================================================================
# Discord Bot - Build & Deploy Script
# Usage: ./deploy.sh <server_user@server_host>
# Example: ./deploy.sh bot_user@192.168.1.100
# =============================================================================

set -euo pipefail

# --- Configuration ---
BINARY_NAME="my_rust_bot"
REMOTE_DIR="/home/bot_user/bot"
SERVICE_NAME="discord-bot"

# --- Validate Arguments ---
if [ $# -lt 1 ]; then
    echo "Usage: $0 <user@host>"
    echo "Example: $0 bot_user@192.168.1.100"
    exit 1
fi

REMOTE_HOST="$1"

echo "=== Building release binary ==="
cargo build --release

echo "=== Uploading binary to server ==="
scp "target/release/${BINARY_NAME}" "${REMOTE_HOST}:${REMOTE_DIR}/${BINARY_NAME}.new"

echo "=== Deploying on server ==="
ssh "${REMOTE_HOST}" << 'EOF'
    cd /home/bot_user/bot

    # Swap binaries atomically
    if [ -f my_rust_bot ]; then
        mv my_rust_bot my_rust_bot.bak
    fi
    mv my_rust_bot.new my_rust_bot
    chmod +x my_rust_bot

    # Restart the service
    sudo systemctl restart discord-bot

    # Show status
    echo "=== Service Status ==="
    sudo systemctl status discord-bot --no-pager
EOF

echo "=== Deploy complete ==="
