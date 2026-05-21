---
id: FEAT--SESSION-LOCK-WINDOWS
phase: 2
type: feat
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: FEAT — Windows Session Locking — cross-platform session write integrity
tags: [msp, sessions, locking, windows, parity, m9f]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--MEMORY-SESSIONS
    - CONCEPT--MSP-ROADMAP
created_at: 2026-05-18T13:30:00+07:00
---

# FEAT — Windows Session Locking

## 1. Summary

This feature implements a robust, cross-platform session locking mechanism using the `proper-lockfile` library. It ensures that only one agent or process can write to a specific session JSONL file at any given time, specifically addressing reliability issues and file system limitations encountered on Windows.

## 2. Motivation

The current session management relies on basic file system checks which are prone to race conditions, especially on Windows where file locking behaviour differs significantly from POSIX systems. Multiple agents attempting to append to the same session log simultaneously can lead to data corruption or silent write failures. `proper-lockfile` provides a proven, stale-lock-aware mechanism that works consistently across Windows and Linux, ensuring session integrity for multi-agent workflows.

## 3. Requirements

### 3.1 Lock Management

- **Lock on Open:** Acquire a lock before opening or creating a session file for writing.
- **Release on Close:** Release the lock immediately after the write operation is completed or the session is closed.
- **Stale Lock Handling:** Automatically detect and remove stale locks (locks left behind by crashed processes) after a defined timeout.

### 3.2 Error Handling

- **Wait/Retry:** Implement a configurable retry logic if a lock cannot be acquired immediately.
- **Fail Gracefully:** If a lock cannot be acquired after retries, the system must report a clear error and prevent the write operation instead of failing silently.

### 3.3 Platform Parity

- The implementation must behave identically on Windows and Linux.
- The lock files should be stored alongside the session files (e.g., `<session_id>.jsonl.lock`).

## 4. Acceptance Criteria

- [ ] Concurrent write attempts to the same session ID from different processes are correctly serialized or gated.
- [ ] No data corruption or interleaved JSON lines observed during stress tests.
- [ ] Stale locks from simulated process crashes are correctly handled.
- [ ] Works seamlessly on both Windows 10/11 and Linux environments.

## 5. Connections

- `[[CONCEPT--MSP-ROADMAP]]` §3 M9f.
- `[[CONCEPT--MEMORY-SESSIONS]]` — the core system this feature protects.
