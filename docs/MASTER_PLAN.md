# MASTER_PLAN — GenesisBlock Engine-Wedge-First Distribution Program

> Phase 0 deliverable. **Status: draft until the owner approves Phase 0.**
> Source of truth for scope: [`docs/adr/ADR--ENGINE-WEDGE-FIRST.md`](adr/ADR--ENGINE-WEDGE-FIRST.md)
> (accepted 2026-07-07). Per RWANG §10, no further document is generated until this plan is approved.

## 0. Framing — this is a right-sized RWANG application (read first)

**Decision:** run RWANG as a *distribution / productization* program over an **existing,
architecturally-frozen engine**, NOT as a greenfield 0→7 architecture build.
- **Reason:** the engine (`genesis-block-native`, ~6k-line Rust core + NAPI/REST/mobile-FFI) is
  already built, tested (34+ test binaries), and its architecture is fixed by the existing
  codebase + prior ADRs/specs. The ADR froze the *strategic* decision. The gap is **packaging +
  distribution**, not architecture.
- **Consequence:** RWANG **Phases 1–5** (System Architecture, Contracts, Multi-Agent Arch,
  Implementation Spec, QA) are **INHERITED-FROZEN** — satisfied by existing artifacts, cited not
  regenerated: `CLAUDE.md`, `docs/C4--GENESISDB-ARCHITECTURE.md`, the `docs/adr/*` set,
  `PROTOCOL--GENESIS-GRAPH-FFI`, `SPEC--MOBILE-SDK`, `BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS`,
  `SPEC--SQLITE-SUBSTRATE-S0-S1`. Any change to them requires an `ARCHITECTURE_CHANGE_REQUEST` (§11.2).
- **Approved amendment:** [`ARCHITECTURE_CHANGE_REQUEST--HQL-P0-BUGFIXES`](ARCHITECTURE_CHANGE_REQUEST--HQL-P0-BUGFIXES.md)
  narrowly unfreezes HQL **P0 grammar** (correctness+exposure defects) for pre-publish landing.
  P1/P2/P3 remain frozen behind GATE-DEMAND-1. Rationale: shipping documented-but-broken behavior
  under GATE-DEMAND-1 would poison the demand signal, and the pre-P0 v1 baseline window closes
  on the next merge to `main`.
- **Rejected alternative:** regenerate all 38 canonical docs for the existing system — rejected as
  over-process on a frozen codebase (violates Operating Principle #1; adds no architectural value).
- **The real path:** Phase 0 (this plan) → **Phase 6** (decompose the ADR's 4 steps into tasks +
  queue) → **Phase 7** (execute wave-by-wave) with one **hard demand gate** after Wave 1.

## 1. Roadmap (RWANG phases mapped to reality)

| RWANG Phase | This program |
|---|---|
| 0 Discovery | **ACTIVE** — this MASTER_PLAN + scope binding (below). |
| 1–5 Architecture→QA | **INHERITED-FROZEN** — cite existing engine + ADRs/specs; no regeneration. |
| 6 Handoff / Task Decomposition | Decompose ADR steps 1–4 into `TASK-####` (§12.2) → `queue/IMPLEMENTATION_QUEUE.json` + `PROJECT_GRAPH.json` + waves. |
| 7 Implementation | Execute waves; each compiles/verifies/reviewed before the next. **Wave 1 → GATE → Waves 2–4.** |

## 2. Dependency Graph

```
W0 HQL-P0 (baseline→merge→bench) ──▶ W1 Publish engine ──▶ [DEMAND GATE: first-10-installs ~1mo] ──▶ W2 Adapter+MCP ──▶ W4 Graphiti driver
                                            │                                                            │                    ▲
                                            └────────────────────────────────────────────▶ W3 REST binary/Docker/SDK-auth ───┘
```
- **W0** (from ACR-HQL-P0): TASK-0000a baseline **MUST** precede TASK-0000b code-merge; TASK-0000c re-benches.
- Baseline window closes on next merge to `main` — W0 is time-critical.
- W2 (adapter) requires W1 (published engine to depend on).
- W4 (Graphiti driver) requires W3 (REST/SDK, since the Python/Graphiti channel rides REST) **and**
  an external check of Graphiti's `GraphDriver` contract vs the HQL subset (adapter-vs-translator).
- **Everything past W1 is gated on the demand signal** — no funding of W2–W4's expensive halves
  until the install signal clears.
- External deps: npm registry + `NPM_TOKEN`; GitHub release matrix (5 target triples); Graphiti
  issue #1240 window; Docker Hub/GHCR.

## 3. Phase Breakdown & Deliverables

| Phase | Deliverables ([domain] slot bound) | Exit criteria |
|---|---|---|
| **0 Discovery** | `MASTER_PLAN.md` (this); scope = ADR--ENGINE-WEDGE-FIRST; **[domain] bind: `22_<PIPELINE>` → `22_DISTRIBUTION_AND_RELEASE_PIPELINE`**; architecture declared inherited-frozen with citations | Scope frozen; domain slot bound; owner approves this plan |
| **1–5** | *(inherited-frozen — citations in §0; not regenerated)* | N/A — frozen baseline |
| **6 Handoff** | `33_TASK_BREAKDOWN.md` (ADR steps→tasks, §12.2 template), `36_TASK_EXECUTION_ORDER.md` (4 waves), `queue/IMPLEMENTATION_QUEUE.json`, `queue/PROJECT_GRAPH.json`, `37_REVIEW_CHECKLIST.md` | Every task independently executable, capability-assigned, no open architectural decisions |
| **7 Implementation** | `src/`/packaging changes per wave; `state/progress.jsonl` + `events.jsonl` | Each wave compiles, verifies, reviewed; **W1 gate decision recorded** before W2 |

**Wave→ADR-step mapping (Phase 7):**
- **W0 = amendment ACR-HQL-P0** — capture v1 baseline; merge HQL P0 defect fixes (SEARCH target meaningful, hybrid `K` exposed, `EF`/`OVERSAMPLE`/`DIRECTION` grammar, rel alternation); re-bench.
- **W1 = ADR step 1** — publish engine: verify `NPM_TOKEN`, tag `v0.2.0`, fire 5-triple release matrix, fix QUICKSTART/dist-tag(`beta`→correct)/SECURITY.md(0.1.0→0.2.0), publish prebuild `optionalDependencies`.
- **W2 = ADR step 2** — extract Rwang `store/genesis-sidecar.mjs`+`knowledge.mjs` → first published path-independent consumer package; make `mcp/server.js` npx-able (bin entry; fix `vectorDim:1536`→1024).
- **W3 = ADR step 3** — REST server as binary + Docker image; SDK auth header support (Python/Go clients vs `api_key_guard`); unauth `/health`; SIGTERM `save_state()`.
- **W4 = ADR step 4** — Graphiti driver, gated on `GraphDriver` contract check (adapter vs translator).

## 4. Milestones

- **M1 (W1):** `npm install @freshair129/gks-genesis-block-native` works for a stranger → embedded bitemporal graph+vector store, no methodology, no orchestrator.
- **GATE (post-M1, ~1 month):** first-10-external-installs signal. *Owner decision:* proceed / pivot / stop before funding W2–W4.
- **M2 (W2):** path-independent adapter package + `npx` MCP memory server usable in any MCP client.
- **M3 (W3):** `docker run` REST server; authed SDKs.
- **M4 (W4):** GenesisBlock as a Graphiti/LangGraph backend (channel live).

## 5. Estimated Complexity (per wave)

| Wave | Complexity | Rationale |
|---|---|---|
| **W0 HQL-P0** | **S/M** | P0 code is already authored in the working tree; W0 work = baseline capture + merge + re-bench; ≤3 working days |
| W1 Publish | **M** | pure packaging/release plumbing; no engine code change; NPM_TOKEN/matrix unverified = the risk |
| W2 Adapter+MCP | **M** | ~600 lines already exist in Rwang; work is de-`G:/`-ing + packaging + npx bin |
| W3 REST/Docker/SDK | **M/L** | Dockerfile (none exists) + graceful shutdown + SDK auth + health route |
| W4 Graphiti driver | **M → L if translator** | adapter if HQL subset fits `GraphDriver`; L if it demands full Cypher |

## 6. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **W0 baseline captured on dirty tree** (P0 already applied) | Med | High | ACR-HQL-P0 §4 mandatory ordering; verify `git status` clean at capture; TASK-0000a `hazard` flag in queue |
| **Post-P0 regression on existing benches** | Low | Med | ACR-HQL-P0 §4 rollback path (revert if p50/recall regress >5% at equal semantic input) |
| **HQL P1/P2/P3 scope creep into W0** | Med | Med | ACR-HQL-P0 §1 scope is explicit; P1 fold requires a new ACR; track-boundary notes in PLAN & SPEC |
| **No external demand** (nobody installs) | Med | High | The whole point of the GATE — publish is cheap; measure before spending |
| `NPM_TOKEN`/release matrix never validated | Med | Med | W1 first sub-task = dry-run the release workflow end-to-end |
| Graphiti `GraphDriver` needs full Cypher (adapter→translator) | Med | Med | Check contract **before** W4 commit; W4 gated |
| Private MSP/GKS runtime repo the audit missed (D: spot-checked only) | Low | High | Owner confirms; if it exists, re-weight (could revive platform path) |
| Methodology (12-stage/H0-6) leaks into the shipped slice | Low | Med | Design invariant: install-and-go slice carries ZERO methodology |
| Snapshot-isolation gap vs SQLite WAL surfaces in a real fleet | Med | Med | Conceded in ADR; expose shipped `events_since`/Merkle, not build MVCC |
| LYRA ROUND4 (falsification of "we have the layer") still pending | Low | Low | Fold in when written; unlikely to reverse the disk audit |

## 7. Review Checkpoints (owner gates)

1. **Approve this MASTER_PLAN** (RWANG §10) → unlocks Phase 6 task decomposition.
2. **Approve `33_TASK_BREAKDOWN` + queue** (Phase 6 review) → unlocks Phase 7 W1.
3. **The DEMAND GATE after W1** — the single most important business decision: real install signal → fund W2–W4, or pivot/stop. Recorded in `state/events.jsonl`.
4. Per-wave review (W2–W4): compiles, verifies, reviewed before next wave.

---
*Owner: approve this plan to proceed to Phase 6 (task decomposition of Waves 1–4). Or flag the private-MSP/GKS-repo caveat first — it is the one input that could re-open the platform path.*
