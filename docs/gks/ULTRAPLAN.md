# GKS — Ultraplan (Roadmap to SOTA Production)

**Document version:** v4 (Post-Pivot, Post-Phase-B Reconcile)
**Last updated:** 2026-05-15 (ICT)
**Reference spec:** `gks/framework/FRAMEWORK--MSP-ARCHITECTURE-V2.md`, `ADR--AGENTIC-MONOREPO-PIVOT`
**Repo state at writing:**
- Monorepo with apps/{android,cli,desktop,ios,mcp,qwen,tui,web} + packages/{gks,msp}
- Root `gks/` atom vault (322 atoms; Phase B of pivot DONE — per-package vaults merged)
- `@freshair129/gks` workspace-internal, version 0.0.0 (standalone publish dropped)
- GKS CHANGELOG at v3.7.0 (historical, pre-pivot); package.json reset to 0.0.0 since publish was retired

---

## What changed since ULTRAPLAN v3

v3 was written for the standalone-publish era ("Ready to publish under `@evaai/gks`"). That assumption no longer holds.

**Locked decisions that supersede v3:**
- `ADR--AGENTIC-MONOREPO-PIVOT` — drop standalone publish; cognitive_system monorepo IS the product
- `ADR--TASK-TRACKING-AT-ORCHESTRATOR` (was "ADR-015") — task tracking lives in MSP, not GKS
- `ADR--GENESIS-GRAPH-AS-GKS-BACKEND` (was "ADR-016" in v3 jargon) — Genesis Graph chosen, Kuzu stays postponed
- Taxonomy v2.3 — flat type-based atom layout, prefix-based IDs (no more numbered ADRs)

**What this means for the roadmap:**
- Phase 6 (Release / npm publish) is RETIRED. There is no more `npm publish` target.
- Changesets setup is NO LONGER NEEDED.
- The remaining engine-side work is: benchmark sweep + Genesis Graph hardening + MSP-side integration that consumes GKS.

---

## Current status snapshot

| Phase | Status | Notes |
|---|---|---|
| 1 Core layers (Atomic/Vector/Episodic/Obsidian) | DONE | |
| 2 Adapters | DONE | |
| 2B Infra Adapters | 90% DONE | Kuzu permanently postponed per `ADR--GENESIS-GRAPH-AS-GKS-BACKEND` |
| 3 Real-Scale Benchmarks | IN PROGRESS | Runners shipped, full sweeps not yet committed |
| 4 Production Hardening | DONE | OTel, circuit breaker, retry, multi-tenancy, cost tracking, schema migration |
| 5 Developer Experience | DONE | MCP server (12 tools), CLI, ADRs |
| 6 Release (standalone npm) | **RETIRED** | Superseded by `ADR--AGENTIC-MONOREPO-PIVOT` |

---

## Phase 2B — Infra Adapters `[90% DONE]`

| Item | Status | File |
|---|---|---|
| B.1 — pgvector backend | DONE | `packages/gks/src/memory/vector/pgvector.ts` |
| B.2 — HNSW in-process backend | DONE | `packages/gks/src/memory/vector/hnsw.ts` |
| B.3a — Postgres graph backend | DONE | `packages/gks/src/memory/graph/pg.ts` |
| B.3b — Kuzu embedded graph | **POSTPONED (perm.)** | superseded by Genesis Graph (`genesis-graph.ts`) |
| B.4 — MCP-stdio transport for Obsidian | DONE | `packages/gks/src/obsidian-mcp-stdio.ts` |
| B.5 — Cross-encoder reranker fixtures | DONE | `packages/gks/src/rerank.ts` |

### New since v3 — Genesis Graph as primary native backend

`packages/gks/src/memory/graph/` now also ships:
- `genesis-graph.ts` — TS-first in-process graph (Cypher v0, JSONL log)
- `cypher-v0.ts` — Cypher subset parser/executor
- `genesis-graph-errors.ts` — typed error surface

See `gks/blueprint/BLUEPRINT--GENESIS-GRAPH-TS-FIRST.md` and `ADR--GENESIS-GRAPH-AS-GKS-BACKEND`.

---

## Phase 3 — Real-Scale Benchmark Sweep `[IN PROGRESS]`

Runners shipped at `packages/gks/benchmarks/`:
- `locomo.ts` `longmemeval.ts` `beam.ts`
- npm scripts: `bench:locomo` `bench:longmemeval` `bench:beam` `bench:sweep` `benchmarks`
- Tiny datasets committed for CI smoke-test; full datasets must be sourced externally

| Item | Status | Blocker |
|---|---|---|
| G3.1 — LoCoMo against full HF dataset | NOT RUN | Need to pull full HF dataset + commit reproducible report |
| G3.2 — LongMemEval (full set) | NOT RUN | Same |
| G3.3 — BEAM @ 10M tokens | NOT RUN | Need compute budget allocation |
| G3.4 — Reproducible benchmark report | RUNNER DONE | `bench:sweep` exists; needs a real run output committed under `benchmarks/results/` |

**Acceptance criteria for closing Phase 3:**
- Single command: `npm run benchmarks` produces JSON + markdown summary
- Embedder model versions pinned in manifest
- Results committed under `packages/gks/benchmarks/results/<date>/` with git SHA recorded

---

## Phase 4 — Production Hardening `[DONE]`

| Item | Status |
|---|---|
| H.1 — Observability (OTel) | DONE — `src/lib/telemetry-setup.ts` |
| H.2 — Resilience (retry + circuit breaker) | DONE — `src/lib/retry.ts`, `src/lib/circuit-breaker.ts` |
| H.3 — Multi-tenancy (namespacing) | DONE |
| H.4 — Cost & token tracking | DONE — `src/lib/cost-tracker.ts`, `src/lib/pricing.ts` |
| H.5 — Schema migration tooling | DONE — `src/lib/schema-version.ts` |

---

## Phase 5 — Developer Experience `[DONE]`

| Item | Status |
|---|---|
| D.1 — GKS as an MCP server | DONE — `packages/gks/src/mcp-server/` (12 tools) |
| D.2 — REPL / CLI | DONE — `gks` binary (init, retain, recall, lookup, status, backlinks, verify-flow, propose-inbound, hotfix, validate) |
| D.3 — Architecture documentation | DONE — ADR set complete; see `gks/adr/` |

**Post-v3 additions (delivered in v3.6.0 / v3.7.0):**
- Nomic local embedder (TH+EN, 768-dim) — `createNomicEmbedder()`
- Claude Code skill layer — 10 slash commands in `.claude/commands/`
- `verify-flow --through-superseded` — chain walks past `status: superseded`
- Backlinks derivation API + `gks backlinks` CLI + `gks_backlinks` MCP tool
- `Status` type now includes `'superseded'`
- `gks propose-inbound` accepts `phase: 6` (post-implementation audit)

---

## Phase 6 — Release `[RETIRED]`

> Superseded by `ADR--AGENTIC-MONOREPO-PIVOT` (2026-05-13).
> `@freshair129/gks` is workspace-internal only. No npm publish. No changesets needed.
> The monorepo itself ships as the product surface (apps/cli, apps/desktop, apps/web, apps/mcp).

Historical: v3.5.x → v3.6.0 (nomic embedder) → v3.7.0 (phase-6 + backlinks + through-superseded) was the last semver release before publish was dropped. CHANGELOG is preserved at `packages/gks/CHANGELOG.md` for record.

---

## Remaining critical path

```
Phase 3 benchmark sweeps   ┐
                           ├─→  GKS engine declared "SOTA-claim ready"
Genesis Graph hardening    ┘

Pivot Phase C (tooling/docs consolidate)  ─→  monorepo canonical layout complete
```

**Sequential ETA:** ~3–5 days to first SOTA-claim benchmark run + Phase C close-out.

Pivot Phase B is DONE (root `gks/` materialised, 322 atoms unified).
Pivot Phase C is the remaining structural work — see `docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md` §PHASE C.

---

## Decision log (carried forward + new)

| Decision | Rationale |
|---|---|
| Cut FalkorDB | SSPL license blocks SaaS, no native bi-temporal support |
| B.3a Postgres tables — primary | Free if B.1 ships first; transactional consistency; SQL CTEs handle BFS |
| B.3b Kuzu — POSTPONED PERMANENTLY | Superseded by Genesis Graph (TS-first, in-process, owns Cypher v0) |
| Skip Neo4j / Apache AGE | Ops weight, license friction, pgvector conflicts |
| **DROP standalone npm publish** | Monorepo-as-product per `ADR--AGENTIC-MONOREPO-PIVOT` |
| **Genesis Graph as canonical native backend** | TS-first, no FFI surface needed for MVP; see `ADR--GENESIS-GRAPH-AS-GKS-BACKEND` |
| **Task tracking → orchestrator (MSP)** | `ADR--TASK-TRACKING-AT-ORCHESTRATOR`; atoms keep doc-types only |

---

## Out of scope (carried from v3)

- Federated / multi-region deployment
- Knowledge distillation (compressing old episodic to gist embeddings)
- Self-hosted reranker fine-tuning
- Real-time streaming retrieval (current model: request/response)
- Web UI / dashboard — `apps/web` exists but is a knowledge-graph explorer, not a GKS control plane

---

## Related plans

- `docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md` — repo restructure (Phase A done, Phase B done, Phase C pending)
- `gks/blueprint/BLUEPRINT--GENESIS-GRAPH-TS-FIRST.md` — Genesis Graph design
- `gks/blueprint/BLUEPRINT--RETRIEVAL-ORCHESTRATION.md` — RRF fusion across recall sources
- `packages/msp/ROADMAP.md` — MSP-side execution plan (consumer of GKS)

---

*v3 of this document is preserved in git history at `docs/gks/ULTRAPLAN.md@<pre-v4-sha>`.*
