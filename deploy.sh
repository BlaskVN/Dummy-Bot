#!/bin/bash
set -euo pipefail

config_file="${1:-.deploy.env}"
if [[ ! -f "$config_file" ]]; then
    echo "Missing deployment config: $config_file (copy .deploy.env.example first)" >&2
    exit 1
fi
if [[ ! -f config.env ]]; then
    echo "Missing runtime config: config.env" >&2
    exit 1
fi

# shellcheck source=/dev/null
source "$config_file"
: "${DEPLOY_REMOTE_HOST:?Missing DEPLOY_REMOTE_HOST}"
: "${DEPLOY_REMOTE_DIR:?Missing DEPLOY_REMOTE_DIR}"
: "${DEPLOY_BINARY_NAME:?Missing DEPLOY_BINARY_NAME}"
: "${DEPLOY_SERVICE_NAME:?Missing DEPLOY_SERVICE_NAME}"

cargo build --release --locked
binary_path="target/release/${DEPLOY_BINARY_NAME}"
binary_sha="$(sha256sum "$binary_path")"
binary_sha="${binary_sha%% *}"
config_sha="$(sha256sum config.env)"
config_sha="${config_sha%% *}"
upload_dir="$(ssh "$DEPLOY_REMOTE_HOST" mktemp -d /tmp/dummy-bot-deploy.XXXXXX)"
scp "$binary_path" "${DEPLOY_REMOTE_HOST}:${upload_dir}/binary"
scp config.env "${DEPLOY_REMOTE_HOST}:${upload_dir}/config.env"

ssh "$DEPLOY_REMOTE_HOST" bash -s -- \
    "$DEPLOY_REMOTE_DIR" "$DEPLOY_BINARY_NAME" "$DEPLOY_SERVICE_NAME" \
    "$upload_dir" "$binary_sha" "$config_sha" <<'REMOTE'
set -euo pipefail
remote_dir="$1"
binary_name="$2"
service_name="$3"
upload_dir="$4"
expected_binary_sha="$5"
expected_config_sha="$6"
if [[ "$upload_dir" != /tmp/dummy-bot-deploy.* ]]; then
    echo "Unsafe deployment upload directory: $upload_dir" >&2
    exit 1
fi
cd "$remote_dir"

if [[ ! -f .env ]]; then
    echo "Missing runtime config: $remote_dir/.env" >&2
    exit 1
fi

sudo chown root:root "$remote_dir"
sudo chmod 0755 "$remote_dir"
stage_dir="$remote_dir/.deploy-staging"
sudo install -d -o root -g root -m 0700 "$stage_dir"
binary_stage="$(sudo mktemp "$stage_dir/${binary_name}.XXXXXX")"
config_stage="$(sudo mktemp "$stage_dir/config.env.XXXXXX")"
cleanup() {
    if [[ -n "$binary_stage" ]]; then
        sudo rm -f "$binary_stage"
    fi
    if [[ -n "$config_stage" ]]; then
        sudo rm -f "$config_stage"
    fi
    find "$upload_dir" -type f -delete
    rmdir "$upload_dir" 2>/dev/null || true
}
trap cleanup EXIT

sudo install -o root -g root -m 0755 "$upload_dir/binary" "$binary_stage"
sudo install -o root -g root -m 0644 "$upload_dir/config.env" "$config_stage"
actual_binary_sha="$(sudo sha256sum "$binary_stage")"
actual_binary_sha="${actual_binary_sha%% *}"
actual_config_sha="$(sudo sha256sum "$config_stage")"
actual_config_sha="${actual_config_sha%% *}"
if [[ "$actual_binary_sha" != "$expected_binary_sha" || "$actual_config_sha" != "$expected_config_sha" ]]; then
    echo "Deployment artifact checksum mismatch" >&2
    exit 1
fi

sudo mv -f "$binary_stage" "$binary_name"
binary_stage=""
sudo mv -f "$config_stage" config.env
config_stage=""
sudo chown root:root .env
sudo chmod 0600 .env
sudo systemctl restart "$service_name"
sudo systemctl status "$service_name" --no-pager
REMOTE
