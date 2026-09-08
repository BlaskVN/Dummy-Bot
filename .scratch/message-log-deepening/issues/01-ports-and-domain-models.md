# Define Ports & Domain Models for Message Logging

Status: resolved

## Overview
Define the ports & adapters abstraction seam and core domain models for message logging.

## Details
- Create `src/message_log/ports.rs`:
  - `MessageLogOutbox` trait: `async fn send_message(...)`, `async fn send_attachment(...)`
  - `AttachmentFetcher` trait: `async fn fetch_attachment(...)`
  - Production implementations: `DiscordOutbox` and `HttpAttachmentFetcher`
  - In-memory test doubles: `InMemoryOutbox` and `MockAttachmentFetcher`
- Create `src/message_log/models.rs`:
  - `MessageLogHealth` enum: `Disabled`, `Healthy`, `Degraded`
  - `MessageLogConfig` struct representing guild config
- Unit tests validating that the test doubles record messages/attachments correctly.

## Comments
- Implemented `MessageLogOutbox` and `AttachmentFetcher` in `src/message_log/ports.rs`.
- Implemented `MessageLogHealth`, `MessageLogConfig`, and `MessageLogOptions` in `src/message_log/models.rs`.
- Verified in-memory doubles and CDN host validations with unit tests.

