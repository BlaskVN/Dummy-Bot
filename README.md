# Dummy Bot

Discord bot viết bằng Rust, Poise/Serenity và SQLite. Mọi tham số vận hành nằm trong `.env`; cấu hình triển khai nằm trong `.deploy.env`.

## Kiến trúc

```text
src/
├── main.rs                 # nạp .env và logging
├── app.rs                  # khởi tạo Discord framework/client
├── state.rs                # state dùng chung của feature
├── config.rs               # parse + kiểm tra toàn bộ cấu hình runtime
├── permissions.rs          # Discord permissions, channel overwrite, role hierarchy
├── database.rs             # pool và truy vấn dùng chung
├── commands/
│   ├── general/            # ping, botinfo, serverinfo
│   ├── moderation/         # kick, ban, purge
│   ├── configuration/      # language, prefix, logging, settings
│   ├── presence.rs
│   └── voice.rs
├── handlers/               # event dispatcher và từng event feature
└── i18n.rs
migrations/                 # SQLx migrations
```

Feature mới chỉ cần thêm module command/handler rồi đăng ký trong `all()` hoặc `dispatch()` tương ứng. Command không còn được phân loại theo tên role (`admin`, `moderator`); Discord permission flags mới là nguồn phân quyền.

## Phân quyền Discord

Command đặc quyền dùng đồng thời:

- `default_member_permissions`: Discord ẩn/chặn slash command ở phía server; chủ guild vẫn có thể tùy chỉnh command permission cho role, user hoặc channel.
- `required_permissions`: Poise kiểm tra lại effective permission khi chạy, áp dụng cho cả slash và prefix command.
- `required_bot_permissions`: từ chối sớm nếu bot thiếu quyền cần thiết.
- Kick/ban kiểm tra role hierarchy của cả người gọi và bot theo đúng thứ tự role Discord.
- Voice và message-log tính effective permission tại channel đích, bao gồm permission overwrite.

| Command | Quyền người dùng | Quyền bot |
|---|---|---|
| `/kick` | `KICK_MEMBERS` + role cao hơn target | `KICK_MEMBERS` + role cao hơn target |
| `/ban` | `BAN_MEMBERS` + role cao hơn target | `BAN_MEMBERS` + role cao hơn target |
| `/purge` | `MANAGE_MESSAGES` | `VIEW_CHANNEL`, `MANAGE_MESSAGES`, `READ_MESSAGE_HISTORY` |
| `/settings`, `/setprefix`, `/messagelog`, `/language` | `MANAGE_GUILD` | theo hành động/channel đích |
| `/connect` | `MOVE_MEMBERS`, phải đang ở voice channel và có `CONNECT` | `VIEW_CHANNEL`, `CONNECT` tại voice channel đó |
| `/disconnect` | `MOVE_MEMBERS` | không cần quyền quản lý thành viên để tự rời |
| `/presence` | application owner trong `OWNER_IDS` hoặc Discord application owner | — |

Tài liệu nền: [Discord permissions](https://docs.discord.com/developers/topics/permissions) và [Poise command checks](https://docs.rs/poise/latest/poise/macros/attr.command.html).

## Cài đặt

Yêu cầu Rust stable và bot token từ Discord Developer Portal.

```bash
cp .env.example .env
# đặt DISCORD_TOKEN trong .env; cấu hình không nhạy cảm nằm trong config.env
cargo run
```

Bot cần bật các privileged intents tương ứng trong Developer Portal: Message Content và Server Members. Voice State intent được yêu cầu cho lệnh voice.

## Cấu hình runtime

`Config::load()` đọc cấu hình vận hành từ `config.env` và secret từ `.env`; thiếu hoặc sai biến sẽ dừng startup với lỗi rõ ràng.

| Nhóm | Biến |
|---|---|
| Secret (`.env`) | `DISCORD_TOKEN` |
| Kết nối (`config.env`) | `DATABASE_URL`, `DATA_DIRECTORY`, `RUST_LOG` |
| Owner/default | `OWNER_IDS`, `DEFAULT_PREFIX`, `DEFAULT_LANGUAGE` |
| Giới hạn command | `PREFIX_MAX_CHARS`, `PURGE_MAX_MESSAGES`, `PURGE_CONFIRMATION_SECONDS`, `BAN_MAX_DELETE_DAYS`, `PRESENCE_MAX_DURATION_MINUTES` |
| Runtime/recovery | `CACHE_MAX_MESSAGES`, `GATEWAY_RESUME_DELAY_SECONDS`, `GATEWAY_READY_DELAY_SECONDS` |
| Message log | `MESSAGE_PREVIEW_CHARS`, `MESSAGE_LOG_CHUNK_CHARS`, `MESSAGE_TIMESTAMP_FORMAT`, `ATTACHMENT_MAX_BYTES`, `PURGE_ATTACHMENT_MAX_TOTAL_BYTES` |
| Giao diện | toàn bộ biến `EMBED_COLOR_*` |

`OWNER_IDS` nhận danh sách Discord user ID phân tách bằng dấu phẩy. Để trống để bot lấy owner/team owner từ Discord application info. Attachment mặc định 10 MiB/file và 64 MiB/purge; hai lượt tải/upload đồng thời được giữ để cân bằng RAM, băng thông và thời gian xử lý. Chỉ tăng khi log guild có giới hạn upload cao hơn và tài nguyên cho phép. Các giới hạn cứng còn lại trong `config::discord_limits` là giới hạn giao thức Discord, không phải tham số triển khai.

Prefix theo guild được lưu trong SQLite và được Poise đọc động; `DEFAULT_PREFIX` chỉ dùng khi guild chưa cấu hình.

## Database migration

Schema nằm trong `migrations/` và được `sqlx::migrate!()` chạy khi startup. Thêm thay đổi schema bằng migration mới; không sửa migration đã chạy trên production.

## Dependency versions

Các direct crate được cố định ở stable release mới nhất đã đối chiếu trên crates.io ngày 2026-08-03:

| Crate | Version | Docs |
|---|---:|---|
| poise | 0.6.2 | [docs](https://docs.rs/poise/0.6.2) |
| tokio | 1.53.1 | [docs](https://docs.rs/tokio/1.53.1) |
| tracing | 0.1.44 | [docs](https://docs.rs/tracing/0.1.44) |
| tracing-subscriber | 0.3.23 | [docs](https://docs.rs/tracing-subscriber/0.3.23) |
| sqlx | 0.9.0 | [docs](https://docs.rs/sqlx/0.9.0) |
| anyhow | 1.0.104 | [docs](https://docs.rs/anyhow/1.0.104) |
| dotenvy | 0.15.7 | [docs](https://docs.rs/dotenvy/0.15.7) |
| chrono | 0.4.45 | [docs](https://docs.rs/chrono/0.4.45) |
| rustls | 0.23.43 | [docs](https://docs.rs/rustls/0.23.43) |

`Cargo.lock` cũng đã được cập nhật toàn bộ dependency bắc cầu. `rustls 0.24.0-dev.0` không được chọn vì là prerelease.

Bot gửi voice-state trực tiếp qua Serenity và không có audio driver, nên chuỗi DAVE/libcrux không nằm trong binary. `rustls-webpki 0.102.8` vẫn bị Serenity 0.12.5 khóa qua Rustls 0.22; chờ Poise/Serenity nâng graph thay vì ép version leaf không tương thích.

## Deploy và systemd

```bash
cp .deploy.env.example .deploy.env
# sửa toàn bộ giá trị DEPLOY_*
./deploy.sh
```

Lần cài đầu, render service từ template bằng đúng `.deploy.env` thay vì sửa/hard-code path trong unit:

```bash
set -a
source .deploy.env
set +a

sed \
  -e "s|@SERVICE_USER@|${DEPLOY_SERVICE_USER}|g" \
  -e "s|@SERVICE_GROUP@|${DEPLOY_SERVICE_GROUP}|g" \
  -e "s|@REMOTE_DIR@|${DEPLOY_REMOTE_DIR}|g" \
  -e "s|@DATA_DIRECTORY@|${DEPLOY_DATA_DIRECTORY}|g" \
  -e "s|@BINARY_NAME@|${DEPLOY_BINARY_NAME}|g" \
  -e "s|@RESTART_SECONDS@|${DEPLOY_RESTART_SECONDS}|g" \
  -e "s|@START_LIMIT_INTERVAL_SECONDS@|${DEPLOY_START_LIMIT_INTERVAL_SECONDS}|g" \
  -e "s|@START_LIMIT_BURST@|${DEPLOY_START_LIMIT_BURST}|g" \
  -e "s|@MEMORY_MAX@|${DEPLOY_MEMORY_MAX}|g" \
  -e "s|@CPU_QUOTA@|${DEPLOY_CPU_QUOTA}|g" \
  systemd/discord-bot.service.template \
  | sudo tee "/etc/systemd/system/${DEPLOY_SERVICE_NAME}.service" >/dev/null

sudo install -d -o "${DEPLOY_SERVICE_USER}" -g "${DEPLOY_SERVICE_GROUP}" -m 0750 "${DEPLOY_DATA_DIRECTORY}"
sudo chown root:root "${DEPLOY_REMOTE_DIR}/.env"
sudo chmod 0600 "${DEPLOY_REMOTE_DIR}/.env"
sudo systemctl daemon-reload
sudo systemctl enable --now "${DEPLOY_SERVICE_NAME}"
```

Theo dõi bằng `journalctl -u "$DEPLOY_SERVICE_NAME" -f`.
