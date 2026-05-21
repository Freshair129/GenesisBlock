---
id: BLUEPRINT--MEMORY-STORAGE-LAYOUT
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Tiered Memory storage and directory layout
tags: [msp, memory, storage, 888, layout]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - SPEC--MEMORY-888
    - CONCEPT--TIERED-MEMORY-DISTILLATION
linked_symbols:
  - file: packages/msp/src/memory/counters.ts
created_at: 2026-05-19T13:00:00+07:00
---

# BLUEPRINT — Memory Storage Layout

## 1. Goal

Define and implement the persistent storage structure for the hierarchical memory system (Sessions, Cores, Spheres), ensuring isolation between namespaces and separation from version-controlled governance atoms.

## 2. Geography

All memory artifacts live under the project's `.brain/` directory (ignored by git).

```text
.brain/
└── msp/
    └── projects/
        └── <ns>/                              # namespace-isolated root
            ├── memory/
            │   ├── counters.json              # cycle state (see T1)
            │   ├── sessions/
            │   │   └── <ulid>.json            # distilled session summary
            │   ├── cores/
            │   │   └── <sphere_seq>_<core_seq>.json
            │   └── spheres/
            │       └── <sphere_seq>.json
            ├── pending/                       # in-flight jobs
            │   ├── raw/
            │   │   └── <ulid>.jsonl           # raw turn logs (T0)
            │   └── distill/
            │       └── <ulid>.tmp             # transient synthesis state
            └── revisions/                     # belief revision records
```

## 3. Tasks

### T1: Counter Management (`packages/msp/src/memory/counters.ts`)

- Implement an atomic counter writer.
- Function: `updateMemoryCounters(ns, type: 'session'|'core'|'sphere')`.
- State: `{ session_seq, core_seq, sphere_seq, total_sessions, last_session_id }`.
- **Enforcement:** Use OS-level advisory locking (via `proper-lockfile` if available, or simple `.lock` file) during counter updates.

### T2: ULID Generator

- Ensure a reliable ULID generator is available for `session_id` to maintain chronological ordering without complex parsing.

### T3: Directory Initialization

- Implement `initMemoryStore(ns: string)` to ensure the required nested directory structure exists.
- Should be called at the start of every session if not already initialized.

### T4: Permission Guards

- Implement a middleware/guard that blocks the LLM's `writeFile` tool from targeting any path containing `.brain/msp/projects/<ns>/memory/`.

## 4. Verification

- [ ] `npm run msp:validate` confirms no memory artifacts are in `gks/`.
- [ ] Script `test-concurrency` simulates 5 concurrent session closes and verifies `counters.json` remains consistent (no lost increments).
- [ ] `Get-ChildItem -Recurse .brain/msp/` matches the specified geography after 10 synthetic sessions.
