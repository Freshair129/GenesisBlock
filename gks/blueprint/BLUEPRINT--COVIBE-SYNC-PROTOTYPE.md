---
id: BLUEPRINT--COVIBE-SYNC-PROTOTYPE
phase: 3
type: blueprint
status: proposed
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — CoVibe Sync Prototype Implementation Plan
tags: [covibe, technical-spike, youtube, websocket, plan, m13]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--COVIBE-SYNC-PROTOTYPE
created_at: 2026-05-22T08:15:00+07:00
---

# BLUEPRINT — CoVibe Sync Prototype

## 1. Goal

Implement a minimal functional prototype to prove YouTube synchronization across multiple clients using Socket.IO.

## 2. Implementation Steps

### T1: Minimal Sync Server (`apps/web/server/sync-server.ts`)
- Use Express + Socket.IO.
- Handlers:
    - `join-room(roomId)`
    - `playback-command(roomId, { action, time, timestamp })`
- Broadcast logic: Forward `playback-command` to all other clients in the same room.

### T2: YouTube Hook (`apps/web/src/hooks/useYouTubeSync.ts`)
- Create a custom React hook to manage the YouTube IFrame Player.
- Features:
    - Initialize `YT.Player`.
    - Function `play(time)`, `pause()`, `seek(time)`.
    - Event emitter for local state changes.

### T3: Sync Component (`apps/web/src/components/covibe/SyncPrototype.tsx`)
- Simple UI with:
    - Room ID input and "Join" button.
    - YouTube video ID input.
    - Status indicators (Connected/Disconnected, Master/Slave).
    - Big "Play/Pause/Sync" buttons for mobile testing.

### T4: Latency Compensation (v0.1)
- When receiving a `play` command:
    - Target time = `receivedTime + (Date.now() - commandTimestamp)`.
    - Allow for a small fixed buffer (e.g. 100ms) to ensure the player is ready.

## 3. Verification Plan

### 3.1 Local Concurrency Test
- Open two browser tabs on the same machine.
- Verify that clicking "Play" in one tab starts the video in the other.
- Measure local drift (should be near 0ms).

### 3.2 Network Drift Test
- Host the server on a accessible machine (e.g. using a tunnel or local IP).
- Connect an iPhone and an Android phone.
- Verify sync quality and document mobile-specific autoplay issues.
