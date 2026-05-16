---
id: CONCEPT--MSP-ROADMAP
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP roadmap — passport-over-Obsidian execution plan M7 → M10
tags:
  - msp
  - roadmap
  - ultraplan
  - planning
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY","ADR--MSP-OBSIDIAN-INTEGRATION","ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS","ADR--GRAPH-IS-GKS-DOMAIN","ADR--EMBEDDING-MODEL-PARITY"]}
created_at: 2026-05-04T07:06:09.872+07:00
---

# CONCEPT — MSP roadmap

> **valid_until**: 2026-08-01 (review quarterly). Each milestone may revise this concept via `update_atomic`.
>
> **Updated 2026-05-04 (M7-prep follow-up)**: post-merge audit against GksV3 3.6.0 reduced M7a effort (wraps existing GKS adapter) and M8d scope (existence checks delegated to `gks validate --links`). Added [[ADR--GRAPH-IS-GKS-DOMAIN]] + [[ADR--EMBEDDING-MODEL-PARITY]] + 4 upstream proposals. See `[[AUDIT--M7-PREP-FOLLOWUP]]`.

## 0. Where we are

| PR | Milestone | Atoms | Tests | Status |
|---|---|---|---|---|
| #1 | M0+M1+M2 — bootstrap + atoms + validator | 5 | 49 | merged |
| #2 | Knowledge base (41 atoms) | 47 | 49 | merged |
| #3 | M3a — pre-commit hook | 53 | 53 | merged |
| #4 | M3 b/c/d — contract loader + 4 FEATs + phase-6 | 64 | 151 | merged |
| #5 | M4 — bin + CI + Ollama + vitest | 70 | 178 | merged |
| #6 | M5 — pre-push + hotfix + anti-hall + required-fields | 84 | 218 | merged |
| #7 | M6 — msp-mcp-server | 89 | 233 | merged |
| #8 | M7-prep — architecture v2 + spec 2.0.0 (this PR) | 95 | 233 | open, CI green |

Foundation complete: validator (12 rules), 5 FEAT scaffolds implemented, MCP server (6 tools), pre-commit + pre-push hooks, CI on Node 20+22, runtime contract loader (`atomic_contract.yaml`), codegen runner with Ollama + vitest acceptance, phase-6 wrapper.

## 1. Near-term — M7 (passport core)

Goal: passport actually **carries** memory + soul + retrieval. Right now writers exist but no orchestration sits on top.

| Sub | Scope | Atoms | Effort | Depends |
|---|---|---|---|---|
| M7a | Wrap GKS `RestObsidianAdapter` + Smart Connections probe + filesystem fallback | 4 | small (was medium — GKS provides adapter) | M7-prep merged |
| M7b | Consolidator (importance scorer + episode-boundary detector + summariser) | 4 | medium | M7a |
| M7c | Retrieval orchestration (`msp_recall` fuse GKS vector + episodic + backlinks via RRF; SC deep-links optional) | 4 | medium-large | M7a, M7b, GKS `createNomicEmbedder` |
| M7d | Context compression (token-budget aware lossy summarisation) | 4 | medium | M7b |
| M7e | Identity / soul layer (`src/identity/` — profile, voice, preferences) | 4 | small | — |
| M7f | MCP tool surface expansion (`msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_*`) | tool atoms | small | M7c-e |

**Deliverable**: `msp_recall("how did we decide rate limiting?")` returns ranked + provenance from Obsidian + episodic merged.

**Test target**: ~+60 → 300 total.

## 2. Mid-term — M8 (governance protocol layer)

Goal: turn `FRAME--*` from descriptive docs into executable contracts via `PROTO--` atoms (currently only id-format reserves the prefix; no actual PROTO atoms exist yet).

| Sub | Scope | Atoms |
|---|---|---|
| M8a | Establish `PROTO--` pattern: scaffold template + `atomic_contract.yaml` extension | 1 ADR + 1 meta-FEAT |
| M8b | `[[PROTO--PHASE-GATES]]` — enforce P0..P6 ordering at PR-time | 4 atoms + impl + CI gate |
| M8c | `[[PROTO--SCALING-LEVEL-GATE]]` — auto-detect L1/L2/L3 from PR diff + check required-atom set | 4 atoms + impl + CI gate |
| M8d | `[[PROTO--ALGO-PARAM-COUPLING]]` — bi-directional `tunes` ↔ `tunable_by` reciprocal validator (existence already checked by `gks validate --links`; MSP only adds the type-pairing constraint) | 4 atoms + impl (smaller scope) |
| M8e | `[[PROTO--AUTHORITY-ENFORCEMENT]]` — git author tier ↔ touched paths | 4 atoms + impl + git ACL bridge |
| M8f | Audit pass: which existing validator rules should be promoted to PROTO atoms? | refactor PR; no new code |

**Deliverable**: PR touching 5 modules but only carrying a typo-fix ADR is blocked at CI ("scope = L3, missing BLUEPRINT/AUDIT").

**Test target**: ~+50 → 350 total.

## 3. Strategic — M9 (lifecycle + distribution)

Goal: address what real-world adoption breaks.

| Sub | Scope | Why |
|---|---|---|
| M9a | Decision atrophy guards — `valid_until` enforcement + scheduled review report | ADRs not updated → enforced lies |
| M9b | Delegation policy ADR — L2 atoms auto-approved by 2 senior; L3 Boss-only | Boss bottleneck > 5 devs |
| M9c | Cross-repo verify-flow — `gks verify-flow --remote=<repo>` | ADRs in repo A that repo B must honour |
| M9d | Migration tooling — `npm run msp:import-notion <export-zip>` | adoption barrier for existing Notion teams |
| M9e | Auto-ADR generator — agent draft ADR from code change automatically | reduce "doc first" friction from 30 min → 30 s |
| M9f | Windows lock for sessions (`proper-lockfile`) | cross-platform parity |

## 4. Scale-up — M10 (vector / graph backends)

Triggered only when Smart Connections or markdown plateau:

| Sub | Trigger |
|---|---|
| M10a | Companion plugin "msp-bridge" — stable Smart Connections API + pgvector adapter | vault > 5,000 atoms or semantic latency > 500ms |
| M10b | Optional Kuzu / Neo4j graph backend | crosslinks > 50,000 or multi-hop on hot path |
| M10c | RRF tuning + retrieval benchmarks | retrieval quality plateaus → empirical comparison |

## 5. Deferred — out of MSP scope (track but don't build here)

| Item | Owner | Trigger |
|---|---|---|
| Sessions/episodic auto-hookup | Agent harness (Claude Code, Cursor, EVA) | M7f lands → harness calls `msp_remember` |
| GKS upstream `phase: 6` patch | GKS maintainers | upstream PR |
| Cross-tenant authorisation | Production deployment / orchestrator | multi-tenant requirement |
| Distributed HOTFIX timer enforcement | Orchestrator + cluster coordinator | multi-machine team |
| Embedder choice | Project owner config | per-project decision |

## 6. Upstream contributions

Drafts live in `upstream/gks-proposals/` (informational; not enforced by MSP CI). MSP cannot push to `Freshair129/GksV3` directly — drafts are for the GKS maintainer to apply manually.

| Target | Item | Draft | Status |
|---|---|---|---|
| `Freshair129/GksV3` | `phase: 6` accept in `propose-inbound` CLI | `upstream/gks-proposals/01-phase-6-acceptance.md` | drafted; `scripts/msp/propose.mjs` workaround in place |
| `Freshair129/GksV3` | `gks verify-flow --through-superseded` flag | `upstream/gks-proposals/02-verify-flow-through-superseded.md` | drafted; supersede chain CI break in PR #8 motivated this |
| `Freshair129/GksV3` | Stable backlinks API (`gks backlinks --emit=jsonl`) | `upstream/gks-proposals/03-backlinks-api.md` | drafted; obsoletes MSP `src/memory/backlinks/` |
| `Freshair129/GksV3` | Document Smart Connections + nomic-embed-text-v1.5 compatibility | `upstream/gks-proposals/04-smart-connections-parity.md` | drafted; companion to `[[ADR--EMBEDDING-MODEL-PARITY]]` |
| Smart Connections plugin | Stable REST endpoint contract | — | M10a depends on this |
| `obsidian-mcp` | `msp-bridge` companion plugin | — | M10a |

## 7. Principles to preserve across the roadmap

1. **Self-contained** — MSP+GKS+Obsidian usable without spinning up a separate orchestrator.
2. **Doc-to-code discipline** — every milestone starts with atoms before code.
3. **Bug recorded in AUDIT** — no INCIDENT/HOTFIX/ISSUE for in-PR fixes (precedent since M2).
4. **3-tier gating** — pre-commit + pre-push + CI; never weakened.
5. **Pluggable not opinionated** — Ollama/Anthropic, Smart Connections/RAG, GKS/neo4j all swappable.
6. **Local-first** — embeddings local, hooks local, validator local; cloud is opt-in.
7. **Versioned governance** — FRAME → PROTO → FEAT → CI gate; every layer versioned.

## 8. Suggested execution order

```
Now → merge PR #8
  ↓
M7a (Obsidian client)        ← prereq for everything M7
  ↓
M7e (identity)               ← small, independent, build confidence
  ↓
M7b (consolidator)           ← needs M7a
  ↓
M7c (retrieval orch.)        ← needs M7a + M7b; the killer feature
  ↓
M7f (MCP tools wrap M7b/c)   ← agents can use it
  ↓
M7d (compressor)             ← polish
  ───────────── M7 done ─────────────
  ↓
M8a (PROTO pattern)          ← unlocks all M8
  ↓
M8b/c/d/e in parallel        ← independent gates
  ↓
M8f (audit refactor)         ← cleanup
  ───────────── M8 done ─────────────
  ↓
M9 in priority order (M9b > M9a > M9e > M9d > M9c > M9f)
  ↓
M10 only when triggered (vault size, latency)
```

## 9. End-state size estimate

| Metric | Now | After M7 | After M8 | After M9 |
|---|---|---|---|---|
| Atoms in `gks/` | 95 | ~120 | ~155 | ~190 |
| Tests | 233 | ~300 | ~350 | ~400 |
| Code LOC | ~8k | ~12k | ~14k | ~16k |
| MCP tools | 6 | ~10 | ~15 | ~17 |
| Validator rules | 12 | 12 | ~20 (PROTO-driven) | ~25 |

## 10. Revision policy

Treat this atom as **temporary** (`valid_until: 2026-08-01`). Each milestone PR may update via `update_atomic` proposal; major scope changes (new milestone family) require a `supersede`. Public-facing summary lives at `ROADMAP.md` (root) — keep that lean and link back here.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--OBSIDIAN-AS-RUNTIME]]
- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[ADR--MSP-OBSIDIAN-INTEGRATION]]
- [[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]

