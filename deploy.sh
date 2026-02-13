#!/bin/bash
# =============================================================================
# Discord Bot - Build & Deploy Script
# Usage: ./deploy.sh <server_user@server_host>
# Example: ./deploy.sh bot_user@192.168.1.100
# =============================================================================

set -euo pipefail

# --- Configuration ---
BINARY_NAME="rust_discord_bot"
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
ssh "${REMOTE_HOST}" << EOF
    cd ${REMOTE_DIR}

    # Swap binaries atomically
    if [ -f ${BINARY_NAME} ]; then
        mv ${BINARY_NAME} ${BINARY_NAME}.bak
    fi
    mv ${BINARY_NAME}.new ${BINARY_NAME}
    chmod +x ${BINARY_NAME}

    # Restart the service
    sudo systemctl restart ${SERVICE_NAME}

    # Show status
    echo "=== Service Status ==="
    sudo systemctl status ${SERVICE_NAME} --no-pager
EOF

echo "=== Deploy complete ==="
