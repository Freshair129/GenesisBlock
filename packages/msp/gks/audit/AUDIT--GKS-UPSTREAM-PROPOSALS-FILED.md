---
id: AUDIT--GKS-UPSTREAM-PROPOSALS-FILED
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: 5 GKS upstream proposals filed (2026-05-07)
tags:
  - msp
  - gks
  - audit
  - upstream
  - handoff
crosslinks: {"references":["AUDIT--TWO-REPO-VALIDATION","ADR--EMBEDDING-MODEL-PARITY"]}
linked_symbols: []
created_at: 2026-05-07T04:00:00.000+07:00
---

# AUDIT — GKS upstream proposals filed

## Scope

Records the 2026-05-07 batch filing of all 5 upstream proposals drafted under `upstream/gks-proposals/` as separate issues against `Freshair129/GksV3`. Closes the HANDOFF P2 priority left over from the v0.4.0 release packet.

## What was filed

| # | Topic | Issue | Proposal file |
|---|---|---|---|
| 01 | `phase: 6` acceptance in `propose-inbound` | [GksV3#32](https://github.com/Freshair129/GksV3/issues/32) | `upstream/gks-proposals/01-phase-6-acceptance.md` |
| 02 | `verify-flow --through-superseded` flag | [GksV3#31](https://github.com/Freshair129/GksV3/issues/31) | `upstream/gks-proposals/02-verify-flow-through-superseded.md` |
| 03 | Stable `gks backlinks` derivation API | [GksV3#30](https://github.com/Freshair129/GksV3/issues/30) | `upstream/gks-proposals/03-backlinks-api.md` |
| 04 | Smart Connections + nomic embedder parity doc | [GksV3#29](https://github.com/Freshair129/GksV3/issues/29) | `upstream/gks-proposals/04-smart-connections-parity.md` |
| 05 | Publish `@freshair129/gks@3.6.0` to npm | [GksV3#28](https://github.com/Freshair129/GksV3/issues/28) | `upstream/gks-proposals/05-publish-3.6.0.md` |

All 5 issues are state `open`, author `Freshair129` (repo owner = relayer), bodies preserved without truncation (verified via `api.github.com/repos/Freshair129/GksV3/issues`).

## How filing was executed

1. A pre-filled HTML helper page (`G:\msp\open-gks-issues.html`) was generated containing all 5 (title, body) pairs as URL-encoded query params pointing at `https://github.com/Freshair129/GksV3/issues/new`.
2. The helper page exposed an "Open all 5 in new tabs" button + per-issue buttons.
3. User reviewed each pre-filled tab and clicked **Submit new issue** themselves (publishing public content requires explicit user action).
4. Issue numbers `#28..#32` confirmed via GitHub REST API; mapping to proposals verified by title match.

## Side-effect updates (this commit)

- All 5 `upstream/gks-proposals/0X-*.md` titles: `🟡 drafted` → `🔵 awaiting upstream review`
- Each proposal file gained a `**Filed upstream**: [GksV3#NN](url) (2026-05-07)` line below the title
- `upstream/gks-proposals/README.md` table — status column updated with linked badges
- `HANDOFF-P2-UPSTREAM-ISSUES.md` submission tracker — all 5 boxes checked, URLs filled

## Side-finding — MSP MCP cwd bug (followup)

While trying to call `msp_propose` for this AUDIT atom via the just-installed MSP MCP server, the call failed with:

```
Error: Cannot find module 'C:\Windows\system32\scripts\msp\propose.mjs'
```

The bin resolves `scripts/msp/propose.mjs` relative to `process.cwd()` (which is `C:\Windows\system32` when launched by Claude Desktop) rather than relative to the `--root=G:\msp` flag. Workaround applied: write atom file directly. Bug to track separately — likely 2-line fix in `src/mcp/handlers/propose.ts` or wherever the spawn happens.

## Next-step gates (per `upstream/gks-proposals/README.md` workflow)

When upstream lands one of these:
1. Bump GKS dep version in `package.json`
2. Replace MSP-side workaround with the upstream call
3. Mark proposal file 🔵 → 🟢 merged upstream + add merge commit hash
4. Write `AUDIT--<topic>-UPSTREAMED` atom recording the migration
5. Move proposal file to `upstream/gks-proposals/merged/` for archival

Most concretely — when **#28 (publish 3.6.0)** lands, `ADR--EMBEDDING-MODEL-PARITY` can drop its "Status note" and the Knowledge Browser (`src/index.ts`, `src/memory.ts`) currently excluded from `tsconfig*.json` can be re-enabled.

## Counts

- Atoms in `gks/`: +1 (this audit)
- Upstream proposals filed: 5 / 5 (100%)
- Proposal status changes: 5× 🟡 → 🔵
- Open issues on `Freshair129/GksV3`: +5 (#28..#32)
- New MSP-side bugs surfaced: 1 (MSP MCP cwd resolution)

## Source

User direction "Submit new issue แล้ว" (2026-05-07) confirming all 5 GitHub submissions completed. AUDIT written autonomously to close out HANDOFF P2.
