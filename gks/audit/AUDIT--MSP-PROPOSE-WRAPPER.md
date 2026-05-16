---
id: AUDIT--MSP-PROPOSE-WRAPPER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M3d msp:propose phase-6 wrapper acceptance audit
tags:
  - msp
  - m3
  - m3d
  - audit
  - propose
  - phase-6
crosslinks: {"references":["FRAMEWORK--PHASE-GOVERNANCE"]}
linked_symbols:
  - {"file":"scripts/msp/propose.mjs"}
  - {"file":"test/scripts/propose.test.ts"}
created_at: 2026-05-03T15:43:36.230+07:00
---

# AUDIT — msp:propose phase-6 wrapper

## Scope

Closes M3d from the [[AUDIT--MSP-VALIDATOR]] / [[AUDIT--KNOWLEDGE-BASE]] backlog. GKS 3.5.6 caps `--phase` at 5 in propose-inbound, but the master spec uses P6 for AUDIT atoms.

## Investigation

Tested whether GKS rejects phase=6 only at propose-time or also at promote/index/validator time:
- propose-inbound: rejects (`InboundQueue: invalid phase 6, must be integer 0..5`)
- inbound promote: accepts (no phase check)
- re-indexer (vendored): accepts (already updated to 0..6)
- validator phase-status rule: accepts (range 0..6)
- gks verify-flow: accepts

Only GKS's CLI-level phase validation needs working around. Wrapper takes the simplest fix.

## Implementation

`scripts/msp/propose.mjs` — small Node script:
1. Parses argv; detects `--phase=6`
2. Calls `npx gks propose-inbound` with `--phase=5`
3. After GKS creates the file, patches the frontmatter to set `phase: 6`
4. All other args pass through unchanged
5. Exit code mirrors GKS

Wired as `npm run msp:propose`.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | phase=5 unchanged | ✅ test #1 |
| 2 | phase=6 → propose at 5 then patches file | ✅ test #2 + manual dogfood (this AUDIT was filed via the wrapper) |
| 3 | invalid input → propagates non-zero exit | ✅ test #3 |
| 4 | All other GKS args (--title, --body, --type) pass through | ✅ implicit; tests use them |

## Test summary

```
test/scripts/propose.test.ts: 3/3 passing
```

## Dogfood

This very AUDIT atom was filed at phase 6 via `npm run msp:propose -- ... --phase=6`. The wrapper printed `✓ patched [[AUDIT--MSP-PROPOSE-WRAPPER]] to phase: 6 (per [[ADR--PATH-ENCODING]] M3d)`. The file landed in inbound at phase 6.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 3/3 tests + dogfood (this audit + 5 sibling M3 audits)
- Date: 2026-05-03

## Connections
- [[FRAMEWORK--PHASE-GOVERNANCE]]

