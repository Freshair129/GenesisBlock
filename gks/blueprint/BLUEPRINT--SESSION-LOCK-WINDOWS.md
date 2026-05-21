---
id: BLUEPRINT--SESSION-LOCK-WINDOWS
phase: 3
type: blueprint
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: BLUEPRINT — Windows Session Locking Implementation Plan
tags: [msp, sessions, locking, windows, plan, m9f]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--SESSION-LOCK-WINDOWS
    - CONCEPT--MEMORY-SESSIONS
linked_symbols:
  - file: packages/msp/src/memory/sessions/writer.ts
created_at: 2026-05-18T13:45:00+07:00
---

# BLUEPRINT — Windows Session Locking

## 1. Goal

Integrate `proper-lockfile` into the MSP session management system to provide robust, cross-platform file locking, specifically for Windows environments.

## 2. Implementation Steps

### T1: Dependency Management

- Install `proper-lockfile` and its types in the `msp` package.
- Command: `npm install proper-lockfile --workspace=packages/msp` and `npm install @types/proper-lockfile --save-dev --workspace=packages/msp`.

### T2: Lock Utility (`packages/msp/src/utils/lock.ts`)

- Create a utility to wrap `proper-lockfile` functionality.
- Functions:
  - `lockSession(filePath: string): Promise<() => Promise<void>>`: Acquires a lock and returns a release function.
  - Implement retry logic with `retries: 5, minTimeout: 100`.
  - Set `stale: 10000` (10 seconds) to auto-expire dead locks.

### T3: Session Writer Integration

- Locate the session writing logic (expected in `packages/msp/src/memory/sessions/writer.ts`).
- Wrap the append operation with the lock utility:

    ```typescript
    const release = await lockSession(sessionFilePath);
    try {
        await appendTurn(sessionFilePath, turn);
    } finally {
        await release();
    }
    ```

### T4: Error Handling and Logging

- Ensure that if a lock cannot be acquired, a meaningful error is logged.
- Add telemetry/logs for lock acquisition times to monitor performance on slow file systems.

## 3. Verification Plan

### 3.1 Concurrent Write Test

- Create a script `packages/msp/test/stress/session-lock.test.ts`.
- Launch 10 parallel processes/promises trying to write different turns to the *same* session ID.
- Verify:
  - No "File busy" or "Access denied" errors on Windows.
  - All turns are present in the final JSONL file.
  - No interleaved or corrupted lines.

### 3.2 Stale Lock Recovery

- Simulate a crash by acquiring a lock and not releasing it.
- Wait for the `stale` timeout.
- Verify that a new process can successfully acquire the lock.
