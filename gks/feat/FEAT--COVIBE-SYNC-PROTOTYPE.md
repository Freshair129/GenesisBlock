---
id: FEAT--COVIBE-SYNC-PROTOTYPE
phase: 2
type: feat
status: proposed
vault_id: default
tier: process
source_type: axiomatic
title: FEAT — CoVibe Sync Prototype — YouTube and WebSocket feasibility spike
tags: [covibe, technical-spike, youtube, websocket, sync, m13]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--COVIBE-VISION
    - CONCEPT--COVIBE-ROADMAP
created_at: 2026-05-22T08:00:00+07:00
---

# FEAT — CoVibe Sync Prototype

## 1. Summary

This technical spike aims to prove the core feasibility of the CoVibe project: synchronizing YouTube playback across two independent web clients using WebSockets. It focus on the low-level technical challenges of latency, drift, and IFrame API limitations.

## 2. Motivation

The entire value proposition of CoVibe (shared music for riders) depends on high-quality synchronization. If the drift between two devices is too large or if YouTube's IFrame API is too restrictive on mobile, the project's roadmap needs to be adjusted early. This spike provides the empirical data required to move into the MVP phase.

## 3. Requirements

### 3.1 YouTube Integration
-   Embed a YouTube IFrame Player using the official JS API.
-   Provide basic controls: Play, Pause, Seek, and Load Video.
-   Capture player state changes (e.g. current time, playback status).

### 3.2 WebSocket Room Logic
-   Implement a minimal Node.js / Socket.IO server.
-   Enable "Room" creation and joining.
-   Broadcast playback events (Play/Pause/Seek) from a "Master" client to a "Slave" client.

### 3.3 Synchronization Engine
-   Implement a basic compensation algorithm for network latency.
-   Target drift: **< 500ms** between devices on a standard 4G/5G connection.
-   Measure and log actual drift for analysis.

### 3.4 Mobile Testing
-   The prototype must be accessible via a public or local-network URL.
-   Must be tested and verified on at least one iOS device and one Android device.

## 4. Acceptance Criteria

-   [ ] A "Master" device can trigger playback on a "Slave" device with < 500ms perceived drift.
-   [ ] Seeking on the Master device correctly updates the Slave device's position.
-   [ ] A report exists summarizing the platform-specific limitations (e.g. Autoplay policies).

## 5. Connections
-   `[[CONCEPT--COVIBE-ROADMAP]]` — Phase 0 task.
