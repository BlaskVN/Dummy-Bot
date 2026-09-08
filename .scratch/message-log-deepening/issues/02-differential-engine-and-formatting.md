# Extract Differential Text & Formatting Engine

Status: resolved
Blocked by: 01

## Overview
Extract formatting, markdown quoting, diff generation, jump link building, and embed pagination into a dedicated module `src/message_log/formatting.rs`.

## Details
- Port and deepen:
  - `escape_markdown`
  - `markdown_quote`
  - `markdown_message`
  - `fits_embed_batch`
  - `message_url`
  - `reply_field`
  - Embed construction helpers for delete, update (before/after diff), bulk delete, and degraded metadata entries.
- Migrate and expand unit tests for diffs and embed limits.

## Comments
- Implemented pure formatting functions and embed builders in `src/message_log/formatting.rs`.
- Unit tests verify line quoting, markdown escaping, jump links, batch embed limits, and purge attachment size budgets.

