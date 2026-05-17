---
id: AUDIT--PR-2-PACKAGE-JSON-RESIDUE
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — PR #50 missed package.json bin + files entries for deleted msp_propose
tags:
  - msp
  - audit
  - cleanup
  - package-json
  - inbound-removal
crosslinks: {"references":["BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION","AUDIT--INBOUND-TO-CANDIDATES-MIGRATION-COMPLETE"]}
created_at: 2026-05-09T16:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — PR #50 package.json residue cleaned up

## What was missed

PR #50 (Phase 3 of `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`) deleted `scripts/msp/propose.mjs` along with the `msp_propose` MCP tool, scripts entries (`msp:propose`, `msp:list`, `msp:promote`), and inbound-related code paths. However it left two residual references in `package.json`:

1. `bin.msp-propose: "./scripts/msp/propose.mjs"` — pointed at a now-deleted file. Any consumer running `npm install` against this package would fail or get a broken symlink.
2. `files[]: "scripts/msp/propose.mjs"` — would be packed into a publish tarball, except the file no longer existed.

Discovered post-merge during a manual smoke test of `npm run msp:master compose ...` (PR-6). `npm install` did not fail in our local dev workflow because we run from source via `tsx`, not from a packed install.

## What this PR fixes

- Remove `bin.msp-propose` line from `package.json`
- Remove `scripts/msp/propose.mjs` from `package.json` `files` array

No code, test, or atom changes beyond this audit.

## Why this didn't show up in PR #50 CI

Phase 3's verification gates (`npm test`, validator, check-links, msp:index) all run against the source tree, not against a published/installed package. CI never invoked `npm pack` or `npm install` against the package itself. The bin entry is harmless when running scripts via the `msp:*` aliases in `package.json` `scripts`, which is what every MSP test uses.

## Correctness verification

- `grep -r "scripts/msp/propose.mjs" package.json` → 0 hits after this PR
- `grep -r "msp-propose" package.json` → 0 hits after this PR (the bin name is gone)
- `npm run msp:master compose [[MASTER--MSP-DOC-TO-CODE]] [[MASTER--ATOM-CONTRADICTION-POLICY]]` → still works (output unchanged from PR-6 smoke test)
- `npm test` — green
- `npm pack --dry-run` would now succeed without missing-file errors (not run in CI; verified locally)

## Lesson

When deleting a script in a future cleanup PR, also grep `package.json` for both `bin.<name>` and `files[]` entries pointing at the script. Add this grep to a future `[[PROTO--PACKAGE-JSON-DEAD-REFS]]` predicate (deferred — not urgent given how rare this class of bug is).

## Source

- Discovery: 2026-05-09 manual smoke test post-PR-6 merge
- Original deletion: PR #50 (`feat: delete msp_propose + inbound infrastructure (Phase 3)`)
- Parent BLUEPRINT: `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`

## Connections
- [[AUDIT--INBOUND-TO-CANDIDATES-MIGRATION-COMPLETE]]

