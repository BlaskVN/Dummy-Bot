#!/bin/bash
set -euo pipefail

config_file="${1:-.deploy.env}"
if [[ ! -f "$config_file" ]]; then
    echo "Missing deployment config: $config_file (copy .deploy.env.example first)" >&2
    exit 1
fi

# shellcheck source=/dev/null
source "$config_file"
: "${DEPLOY_REMOTE_HOST:?Missing DEPLOY_REMOTE_HOST}"
: "${DEPLOY_REMOTE_DIR:?Missing DEPLOY_REMOTE_DIR}"
: "${DEPLOY_BINARY_NAME:?Missing DEPLOY_BINARY_NAME}"
: "${DEPLOY_SERVICE_NAME:?Missing DEPLOY_SERVICE_NAME}"

cargo build --release
scp "target/release/${DEPLOY_BINARY_NAME}" \
    "${DEPLOY_REMOTE_HOST}:${DEPLOY_REMOTE_DIR}/${DEPLOY_BINARY_NAME}.new"

ssh "$DEPLOY_REMOTE_HOST" bash -s -- \
    "$DEPLOY_REMOTE_DIR" "$DEPLOY_BINARY_NAME" "$DEPLOY_SERVICE_NAME" <<'REMOTE'
set -euo pipefail
remote_dir="$1"
binary_name="$2"
service_name="$3"
cd "$remote_dir"

if [[ ! -f .env ]]; then
    echo "Missing runtime config: $remote_dir/.env" >&2
    exit 1
fi

sudo rm -f "${binary_name}.installed"
sudo install -o root -g root -m 0755 "${binary_name}.new" "${binary_name}.installed"
sudo mv -f "${binary_name}.installed" "$binary_name"
rm -f "${binary_name}.new"
sudo chown root:root .env
sudo chmod 0600 .env
sudo systemctl restart "$service_name"
sudo systemctl status "$service_name" --no-pager
REMOTE
