# Deepen Guild Time Zone Module

Status: resolved

## Overview
Implement persistence, IANA invariant checking, and next session expiry calculation in `src/timezone.rs`.

## Details
- Implement `get_timezone`, `get_timezone_name`, `set_timezone`, `clear_timezone`, `next_session_expiry`.
- Add integration and unit tests for valid/invalid IANA names, upsert, clear, and 05:00 DST boundary calculations.
