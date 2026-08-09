# Dummy Bot

[English](README.md)

Dummy Bot là Discord bot viết bằng Rust, Poise/Serenity và SQLite. Cấu hình vận
hành nằm trong `config.env`, secret nằm trong `.env`, và cấu hình triển khai nằm
trong `.deploy.env`.

## Tính năng

- Các lệnh thông tin bot và server
- Kick, ban và purge với kiểm tra quyền Discord
- Cấu hình prefix, ngôn ngữ và message log theo từng guild
- Log tin nhắn bị xóa/chỉnh sửa, có lưu attachment với giới hạn tài nguyên
- Lưu trạng thái presence và tự kết nối lại voice channel
- Múi giờ theo guild và hướng dẫn một lần khi cài đặt
- Thông tin ủng hộ do Chủ Bot quản lý với ảnh QR được lưu ổn định
- Vụ việc kiểm duyệt bất biến, kênh kiểm duyệt riêng và lệnh warn/timeout
- Theo dõi Discord AutoMod tùy chọn với đề xuất xem xét có giới hạn

## Yêu cầu và chạy local

Cài Rust stable và tạo bot trong Discord Developer Portal. Bật hai privileged
intent Message Content và Server Members; lệnh voice cũng cần Voice States
intent.

Đặt `MESSAGE_CONTENT_ENABLED=false` khi chưa bật Message Content trong
Developer Portal. Bot vẫn khởi động và đánh dấu Message Log đang bật là suy
giảm; giữ `true` cho deployment hiện có đã được cấp quyền.

```bash
cp .env.example .env
# Đặt DISCORD_TOKEN trong .env. Giá trị không nhạy cảm nằm trong config.env.
cargo run
```

Chạy các bước kiểm tra trước khi merge:

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

## Cấu trúc dự án

```text
src/
├── main.rs                 # nạp môi trường và cấu hình tracing
├── app.rs                  # khởi tạo Discord framework/client
├── state.rs                # state dùng chung của feature
├── config.rs               # parse và kiểm tra cấu hình runtime
├── permissions.rs          # permission, overwrite và role hierarchy
├── database.rs             # khởi tạo pool và truy vấn dùng chung
├── commands/
│   ├── general/            # ping, botinfo, serverinfo
│   ├── moderation/         # kick, ban, purge
│   ├── configuration/      # language, prefix, logging, settings
│   ├── presence.rs
│   └── voice.rs
├── handlers/               # event dispatcher và từng event feature
└── i18n.rs                 # chuỗi EN/VI/JA hiển thị cho người dùng
migrations/                 # SQLx migrations được nhúng vào binary
systemd/                    # service template dùng khi triển khai
```

Khi thêm command hoặc handler, tạo module trong thư mục tương ứng rồi đăng ký
vào hàm `all()` hoặc `dispatch()` gần nhất. Phân quyền dựa trên Discord
permission flags, không dựa trên tên role.

## Cấu hình runtime

`Config::load()` đọc cấu hình không nhạy cảm từ `config.env` và bot token từ
`.env`. Thiếu hoặc sai giá trị sẽ dừng startup với lỗi rõ ràng.

| Nhóm | Biến |
|---|---|
| Secret (`.env`) | `DISCORD_TOKEN` |
| Kết nối | `DATABASE_URL`, `DATA_DIRECTORY`, `RUST_LOG` |
| Owner/mặc định | `OWNER_IDS`, `DEFAULT_PREFIX`, `DEFAULT_LANGUAGE` |
| Giới hạn command | `PREFIX_MAX_CHARS`, `PURGE_MAX_MESSAGES`, `PURGE_CONFIRMATION_SECONDS`, `BAN_MAX_DELETE_DAYS`, `PRESENCE_MAX_DURATION_MINUTES` |
| Runtime/recovery | `CACHE_MAX_MESSAGES`, `GATEWAY_RESUME_DELAY_SECONDS`, `GATEWAY_READY_DELAY_SECONDS` |
| Message log | `MESSAGE_CONTENT_ENABLED`, `MESSAGE_PREVIEW_CHARS`, `MESSAGE_LOG_CHUNK_CHARS`, `MESSAGE_TIMESTAMP_FORMAT`, `ATTACHMENT_MAX_BYTES`, `PURGE_ATTACHMENT_MAX_TOTAL_BYTES` |
| Giao diện | toàn bộ biến `EMBED_COLOR_*` |

`OWNER_IDS` nhận danh sách Discord user ID phân tách bằng dấu phẩy. Để trống để
dùng owner/team owner của Discord application. Attachment mặc định bị giới hạn
10 MiB/file và 64 MiB/purge; chỉ tăng khi guild đích và tài nguyên host hỗ trợ.

Prefix theo guild được lưu trong SQLite. `DEFAULT_PREFIX` chỉ được dùng cho đến
khi guild lưu prefix riêng.

## Database migration

Migration là cần thiết vì cấu hình guild và presence lâu dài được lưu trong
SQLite. Migration được nhúng vào binary và `sqlx::migrate!()` tự chạy khi
startup.

- Không sửa migration có thể đã chạy ở môi trường hiện hữu; SQLx kiểm tra
  checksum của file.
- Chỉ thêm file SQL đánh số mới khi schema thay đổi.
- Không thêm migration rỗng cho release chỉ đổi code ứng dụng.
- Kiểm tra thay đổi schema bằng
  `cargo test database::tests::applies_initial_migration`.
- Backup database SQLite trước khi deploy migration có thay đổi phá hủy dữ liệu.

Các bản phát hành hiện tại áp dụng migration được đánh số theo thứ tự;
`0001_initial.sql` vẫn là mốc nâng cấp và không được chỉnh sửa.

## Phân quyền Discord

Command đặc quyền kết hợp kiểm tra của Poise với metadata quyền command của
Discord. Kick và ban còn kiểm tra role hierarchy của người gọi và bot. Voice và
message log tính quyền tại channel đích, bao gồm channel overwrite.

| Command | Quyền người dùng | Quyền bot |
|---|---|---|
| `/kick` | `KICK_MEMBERS` và role cao hơn target | `KICK_MEMBERS` và role cao hơn target |
| `/ban` | `BAN_MEMBERS` và role cao hơn target | `BAN_MEMBERS` và role cao hơn target |
| `/purge` | `MANAGE_MESSAGES` | `VIEW_CHANNEL`, `MANAGE_MESSAGES`, `READ_MESSAGE_HISTORY` |
| `/warn`, `/timeout`, `/case view`, `/case list` | `MODERATE_MEMBERS` và hierarchy của target | `MODERATE_MEMBERS` và hierarchy của target với timeout |
| `/settings`, `/setprefix`, `/messagelog`, `/language`, `/timezone`, `/moderation-channel`, `/automod-observer`, `/case void` | `MANAGE_GUILD` | tùy hành động/channel đích |
| `/donation` | Chủ Bot | — |
| `/connect` | `MOVE_MEMBERS`, đang ở voice channel và có `CONNECT` | `VIEW_CHANNEL`, `CONNECT` tại channel đó |
| `/disconnect` | `MOVE_MEMBERS` | không cần quyền quản lý thành viên để tự rời |
| `/presence` | owner trong `OWNER_IDS` hoặc Discord application owner | — |

## Release và triển khai

Khi release, cập nhật version trong `Cargo.toml`, chạy toàn bộ kiểm tra ở trên,
commit vào `main`, rồi tạo annotated tag `vX.Y.Z` trên commit đó. Luôn commit
`Cargo.lock`.

Quy trình deploy build tại máy local, thay thế nguyên tử binary và cập nhật
`config.env` trên server:

```bash
cp .deploy.env.example .deploy.env
# Đặt toàn bộ giá trị DEPLOY_*.
./deploy.sh
```

Lần cài đầu cần thêm systemd unit. Nạp `.deploy.env`, render
`systemd/discord-bot.service.template` bằng cách thay các placeholder `@...@`,
cài file vào `/etc/systemd/system`, tạo `DEPLOY_DATA_DIRECTORY`, rồi chạy:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now "$DEPLOY_SERVICE_NAME"
journalctl -u "$DEPLOY_SERVICE_NAME" -f
```

Giữ `.env` trên server thuộc root với mode `0600`. `deploy.sh` chủ động không
upload secret.

## Bảo trì dependency

`Cargo.toml` là nguồn chính cho direct dependency và `Cargo.lock` khóa toàn bộ
build. Đọc release note trước khi nâng Poise/Serenity, SQLx, Rustls hoặc Tokio,
sau đó chạy lại toàn bộ chuỗi kiểm tra. Bot gửi voice state trực tiếp qua
Serenity và không dùng audio driver.
