---
id: CONCEPT--COVIBE-ROADMAP
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — CoVibe Roadmap — 4-Phase execution plan
tags: [covibe, roadmap, execution, mvp]
aliases: [sprint-plan, milestones]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - CONCEPT--COVIBE-VISION
created_at: 2026-05-20T11:45:00+07:00
---

# CONCEPT — CoVibe Roadmap

## 1. Overview

The CoVibe project follows a 4-phase rollout strategy, moving from initial technical feasibility to a private beta for real-world riders.

## 2. Phases

### Phase 0: Feasibility Spike (3–5 Days)
- **Goal:** Prove YouTube IFrame + WebSocket sync works on 2 mobile devices.
- **Tasks:** Prototype player, basic room logic, drift measurement (< 500ms target).

### Phase 1: MVP Core (2 Weeks)
- **Goal:** Enable Rider to create a room and Passenger to join via QR.
- **Sprint 1A:** Project setup (React + Vite), Backend (Node + Socket.IO), QR generator.
- **Sprint 1B:** YouTube link parser, queue management, basic playback sync.

### Phase 2: Hardening + Rider UX (2 Weeks)
- **Goal:** Stability and Rider-specialized UI.
- **Sprint 2A:** Drift correction algorithm, latency compensation, reconnect logic.
- **Sprint 2B:** Large-button Rider dashboard, OLED saver, analytics, error tracking.

### Phase 3: Private Beta (2–3 Weeks)
- **Goal:** Test with 20–50 real rider pairs.
- **Tasks:** Onboarding flow, feedback loops, bug fixing, retention analysis.

## 3. Post-Beta Expansion (Backlog)
- Hotspot / Local-only mode.
- Intercom voice chat.
- Convoy GPS tracking.
- Native apps for background playback.

## Connections
- `[[CONCEPT--COVIBE-VISION]]` — the guiding vision.
