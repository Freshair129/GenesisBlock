# GKS upstream proposals

Drafts of patches that the MSP project would like to see in
[`Freshair129/GksV3`](https://github.com/Freshair129/GksV3). MSP cannot push
directly to that repo — these are informational, for the GKS maintainer to
adapt + apply.

Each file is **one focused change** with:
- **Why** — the MSP-side pain point
- **What** — the proposed code change (file + diff sketch)
- **Compat** — backward-compat story
- **Test** — what to add to vitest
- **Atom reference** — the MSP-side atom that drove this

Status legend in titles: 🟡 drafted, 🔵 awaiting upstream review, 🟢 merged upstream.

## Proposals

| # | File | Topic | Status |
|---|---|---|---|
| 01 | `01-phase-6-acceptance.md` | Accept `phase: 6` in `gks propose-inbound` | 🔵 [GksV3#32](https://github.com/Freshair129/GksV3/issues/32) |
| 02 | `02-verify-flow-through-superseded.md` | `gks verify-flow --through-superseded` flag | 🔵 [GksV3#31](https://github.com/Freshair129/GksV3/issues/31) |
| 03 | `03-backlinks-api.md` | Stable backlinks derivation API (`gks backlinks --emit=jsonl`) | 🔵 [GksV3#30](https://github.com/Freshair129/GksV3/issues/30) |
| 04 | `04-smart-connections-parity.md` | Document Smart Connections + nomic-embed-text-v1.5 compatibility | 🔵 [GksV3#29](https://github.com/Freshair129/GksV3/issues/29) |
| 05 | `merged/05-publish-3.6.0.md` | Publish `@freshair129/gks@3.6.0` to npm | 🟢 [GksV3#28](https://github.com/Freshair129/GksV3/issues/28) — published 2026-05-07 |
| 06 | `merged/06-msp-relationship-update.md` | Update `docs/MSP_RELATIONSHIP.md` — MSP migrated off inbound queue (Phase 3); agent-agnostic reframe | 🟢 [GksV3#22f1751](https://github.com/Freshair129/GksV3/commit/22f1751) — merged 2026-05-11 |
| 07 | `merged/07-frame-broader-definition.md` | Broaden `FRAME--` definition (or add `METHODOLOGY--`) — canonical narrow vs Memory-OS practice | 🟢 [GksV3#22f1751](https://github.com/Freshair129/GksV3/commit/22f1751) — merged 2026-05-11 |

## How to submit upstream

→ **[`SUBMISSION.md`](./SUBMISSION.md)** — copy-paste-ready issue bodies for relay to `Freshair129/GksV3`. Now covers proposals #01–#04 (filed) and #06–#07 (merged upstream).

## Why these live in MSP repo

GKS's storage-engine scope (per `SCOPE.md`) is intentionally narrow. These
proposals are MSP-driven — they exist because MSP-shaped use cases hit gaps.
Putting them here means:

1. The proposal context (which MSP atom motivated it) is preserved.
2. MSP tracks "what we want from GKS" vs "what we're working around".
3. Once upstream lands, the MSP-side workaround can be removed and an
   `AUDIT--` atom records the migration.

## Workflow when an upstream lands

1. GKS releases version with the change.
2. Bump `package.json` GKS dep to that version.
3. Replace MSP workaround with the upstream call.
4. Mark proposal file `status: 🟢 merged upstream` + add merge commit hash.
5. Write `AUDIT--<topic>-UPSTREAMED` atom recording the migration.
6. Move file to `upstream/gks-proposals/merged/` for archival.
