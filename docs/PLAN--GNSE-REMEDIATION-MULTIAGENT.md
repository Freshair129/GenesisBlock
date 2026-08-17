---
status: draft
---

# PLAN — GNSE Remediation via Multi-Agent Workflow (GoVibe-Orchestrated)

**Status:** Draft (2026-08-17) · **Execution framework:** GoVibe `RUNBOOK-GoVibe-Multi-Agent` (D:\GoVibe\.agents) + `R10-Complexity-Based v2.0` + tiered-swarm skill
**Scope of remediation:** the GNSE thin slice agreed 2026-08-17 — History ADR → journal fix (framed + frontier + tail-replay) → additive tx-time → G3 bench gate. Full 6-tranche GNSE (segment stores, page cache, epoch-HNSW, SQLite demotion) stays a **deferred backlog gated on evidence** (§8).
**Parents:** [ADR--GENESISDB-TEMPORAL-MODEL](adr/ADR--GENESISDB-TEMPORAL-MODEL.md) (candidate — tx-time prior art) · [SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1](SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1.md) (rejects new storage engine in v1 — this plan stays inside that line) · [BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS](BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md) (tx-time probe = mandatory gate) · [ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE](adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md)

**Verified facts this plan is built on** (8-agent code fact-check, 2026-08-17):
- Recovery is either/or (`src/lib.rs:4751`): if `state.json` parses, the WAL is never replayed → fsynced+acked writes after the last `save_state` are lost on reopen. **P0 defect.**
- No tx-time axis anywhere; Query IR temporal = `valid_at` only (`deny_unknown_fields`); `as_of` before a supersession **hides** the node instead of resolving the old version; superseded versions live only as WAL lines that checkpoint compaction destroys.
- Three uncoordinated clocks (commit_sequence / Lamport u32 / WAL file order); plain WAL lines carry no sequence number.
- `caused_by` on supersede is caller-supplied, not auto-chained. `EdgeOutput.recorded_at` is written, never read.

---

## 1. Orchestration model mapping

```
User Intent ─▶ MAIN ORCHESTRATOR (Opus, classifier-only, Delegate Mode)
                 ├ T  Tier      ─ who works (T3/T2/T1.5/T1/T0 per tiered-swarm ladder)
                 ├ C  Complexity ─ C-0..C-3 per R10 (declared in every task reply)
                 ├ H  Scope      ─ path globs the packet may touch = the 🔒 file-lock declaration
                 ├ R  Radius     ─ what the context pack must contain
                 ├ D  Resolution ─ compaction level of that pack (digest vs raw)
                 ├ W  Fan-out    ─ concurrent workers inside the packet
                 └ Budget+Risk   ─ token budget class (S/M/L) + risk class (L/M/H)
              ─▶ Context Pack (Explorer → Context Artifact)
              ─▶ Planner/Architect (Fable) → Plan Artifact + Execution DAG
              ─▶ Ready Frontier → Workers (T2/T1.5/T1) → Patch Artifacts (PRs)
              ─▶ Verifier → Verification Artifact (verify_command output + evidence)
              ─▶ Acceptance Criteria → Final Gate (Lead merge; USER for C-3/H≥3)
              ─▶ Integration Artifact (squash-merged PR + doc/status sync)
```

Role → artifact contract (every packet produces exactly these, enforced by the `TaskCompleted` hook):

| Role | Artifact | Concrete form here |
|---|---|---|
| Explorer | Context Artifact | facts digest + file:line evidence list (D-level: digest-only, no raw code > 50 lines) |
| Planner | Plan Artifact | packet plan with `verify_command`, H-glob lock list, DAG edges |
| Worker | Patch Artifact | PR on `GVBR-{n}-{name}-{agent}` branch; never merges |
| Verifier | Verification Artifact | `verify_command` output + re-runnable evidence commands |
| Final Gate | Integration Artifact | squash merge by Lead; USER sign-off on C-3 packets |

**tiered-swarm HARD RULE applies:** a packet is cheap-routable (T1/T1.5/T0) **only if** it carries a machine-checkable `verify_command`. WAL/recovery/durability packets are **never** cheap-routed regardless of gate (high-risk domain override) — minimum T2 with T3 review.

## 2. Global constraints (read before claiming any packet)

1. **`src/lib.rs` is one file → file-level locks serialize engine packets.** Only one engine-mutating packet may be `in_progress` at a time until WP-0.2 (module split) lands. Docs, tests/, benchmark/, and dashboard packets parallelize freely.
2. **Every PR must keep green:** `cargo test` (all bins), `npm test`, `cargo test --test napi_rest_parity_tests` (declare every new napi method), doc validator (`python scripts/validate_doc_status.py`), mobile feature builds compile (`cargo build --no-default-features --features mobile` — CI).
3. **On-disk format changes carry the GBP1 invariant:** legacy readers kept ≥ 2 releases; `SCHEMA_VERSION` is shared desktop/mobile — one bump, both surfaces.
4. **Branch/PR flow per runbook:** `GVBR-{n}-{name}-{agent}` → PR → CI → Lead review → (USER review for C-3) → squash merge. No direct push to main.
5. **R10 enforcement:** every task reply opens with `Complexity: C-X | Context: H-Y`; C-2+ requires spec/impact analysis before code; `TaskCompleted` blocks completion if required artifacts are missing.

## 3. Work packets

Columns: **C** complexity · **H** access scope (= lock declaration) · **R** retrieval radius (context pack) · **D** pack resolution · **W** fan-out · **T** tier route · **B/R** budget/risk.

### Phase 0 — Governance (Epic H3: "History ADR")

| WP | Deliverable | C | H (lock) | R (context pack) | D | W | T | B/R |
|---|---|---|---|---|---|---|---|---|
| **0.1** | `ADR--GENESISDB-JOURNAL-HISTORY`: resolves WAL-compaction-vs-history-vs-rebuildability; adopts GNSE §18 authority table + invariants I1–I8; defines retention/archive policy answering the 20GB@1M math | **C-3** | `docs/adr/**` only | GNSE fact-check digest; `compact_unlocked` + `wal_checkpoint` excerpts; the 5 authority docs; TEMPORAL-MODEL candidate ADR; mobile disk-budget rows | digest | 1 author + 3-lens review panel (correctness / mobile-disk / CRDT-sync) | T3 author (Fable/Opus), panel T2 | M / L |
| **0.2** *(enabler, optional — decide at Sprint-0 review)* | Mechanical module split of `src/lib.rs` → `src/storage/{journal,graph,vector,temporal,projection}.rs`, zero behavior change | C-2 | `src/**` (**global lock** — schedule alone) | C4 map; CLAUDE.md build/tests | raw | 1 | T2 (mechanical but Rust-visibility-sensitive) | M / M |

**Phase-0 gate:** ADR status `accepted` requires Lead + **USER approval** (R10 C-3 rule). `verify_command`: `python scripts/validate_doc_status.py` + link check. Human sign-off is the real gate → not cheap-eligible.

> **Sprint-0 review outcome (2026-08-17):** ✅ WP-0.1 **ACCEPTED** (3-lens panel APPROVE-WITH-CHANGES ×3, draft-2 incorporated all findings, USER approved). ✅ WP-1.1 **LANDED** (PR #100 → main `4072cc9`). WP-0.2 **DEFERRED** — engine packets are lock-serialized by design through Phase 1-2, so the split buys no parallelism until after; revisit post-Phase-1. Sprint 1 open: WP-1.2 spec = [SPEC--GENESISDB-JOURNAL-FORMAT-V1](SPEC--GENESISDB-JOURNAL-FORMAT-V1.md).

### Phase 1 — Journal (Epic H3: "Journal that keeps history") — branch per packet, strictly sequenced after 1.1

| WP | Deliverable | C | H (lock) | R | D | W | T | B/R |
|---|---|---|---|---|---|---|---|---|
| **1.1** | ✅ **LANDED** (main `4072cc9`, RCA--WAL-TAIL-REPLAY — byte-positional `wal_frontier` (bytes+sha256) + tail replay; WP-1.2 migrates the cursor to `(commit_seq, segment_id, offset)` per ADR D5). Original scope: **P0 fix: WAL tail-replay on startup** (spawned task `task_801c499c`). Failing test first; frontier/offset marker in `state.json`; loader replays WAL tail > frontier. Does NOT change checkpoint semantics | C-2 | `src/lib.rs` (open/try_load_state/save_state manifest), `tests/wal_tail_replay_tests.rs` | loader + writer-thread excerpts; crash-ordering contract (lib.rs:8232-8251); `wal_compaction_tests` | digest+raw(loader) | 1 worker + 1 verifier | **T2 + T3 review + Auditor** (durability) | M / **H** |
| **1.2** | Framed binary journal: length-prefixed frames, CRC32C, **per-frame `commit_seq` stamp** from one unified counter on the single-writer thread; legacy JSONL reader retained (GBP1, ≥2 releases); `events_since`/replay/projection-replay updated | **C-3** (on-disk format → Text→Doc→Diagram→Code) | `src/lib.rs` (WAL writer/readers), `docs/` (format spec + diagram), tests | WAL writer thread; SignedEvent; CRDT sync read path (`events_since`, PushDelta, `persist_signed`); GBP1 invariant | digest | 1 worker; test authoring fans out W=3 (cheap-eligible) | T2 impl; T1.5 tests w/ `verify_command`; T3 format review | L / **H** |
| **1.3** | Frontier checkpoint: seal→archive segment instead of truncate; retention/archive tiers per ADR 0.1; startup = snapshot + tail-replay > frontier (completes I8); mobile foreground-compact path kept bounded | **C-3** | `src/lib.rs` (checkpoint/compaction), tests, `benchmark/` disk-growth probe | WP-1.2 format spec; `compact_unlocked`; mobile SPEC disk rows; 20GB@1M measurement | digest | 1 worker + 1 bench runner (parallel, cheap-eligible) | T2 impl; T1 bench runner | L / **H** |

**Phase-1 gate (machine-checkable):** `cargo test` all green incl. new `wal_tail_replay_tests`, `wal_format_tests` (round-trip + legacy JSONL load), `crdt_sync_tests`; disk-growth probe shows bounded on-device size under retention policy; Auditor confirms no acked-write-loss scenario remains (crash matrix: kill after ack, before save_state; kill mid-seal; kill mid-swap).

### Phase 2 — Additive tx-time (Epic H3: "Bitemporal for real") — depends on 1.2

| WP | Deliverable | C | H (lock) | R | D | W | T | B/R |
|---|---|---|---|---|---|---|---|---|
| **2.1** | `node_versions` projection (SQLite): version chain per entity keyed by `commit_seq`; supersede writes closed version; read API `get_versions(id)` / resolve-at-commit | C-2 | `src/lib.rs` (supersede/projection), tests | WP-1.2 commit_seq contract; projection schema code; TEMPORAL-MODEL ADR | digest | 1 | T2 | M / M |
| **2.2** | Query IR v-next: `temporal.tx_as_of` behind version gate + capabilities endpoint; REST + napi + FFI wiring; `as_of` semantics fix — resolve historical version instead of hiding superseded nodes | C-2 | `src/lib.rs`, `src/router.rs`, `index.d.ts`, `src/ffi.rs`, tests | Query IR spec + parity mappings; temporal filter code; napi_rest_parity rules | digest | 1 worker + parity-table author (cheap-eligible W=2) | T2 impl; T1 parity tables | M / M |
| **2.3** | Small semantics fixes: `caused_by` auto-chain on supersede (default to prev version when caller passes None); `recorded_at` made queryable | C-1 | `src/lib.rs` (supersede_node, filters), tests | supersede + filter excerpts | digest | 1 | T2 (touches supersede — not worth cheap-routing) | S / M |

**Phase-2 gate:** `temporal_queries_tests` extended with tx-axis cases green; `napi_rest_parity_tests` + Query IR parity mappings green; capabilities endpoint advertises `tx_as_of`; **the BENCH-SPEC tx-time probe now passes** (it fails deterministically today — this is the whole point).

### Phase 3 — Evidence gate (Epic H3: "Prove or kill") — 3.1 starts in parallel with Phase 2 (spec-first)

| WP | Deliverable | C | H (lock) | R | D | W | T | B/R |
|---|---|---|---|---|---|---|---|---|
| **3.1** | Bitemporal correctness suite (per interview ROUND2): valid×tx matrix scenarios, correction-after-the-fact, audit reconstruction — written against the WP-2.2 contract before impl lands (phase-scale TDD RED) | C-2 | `tests/**`, `benchmark/**` only (no engine lock) | tx_as_of contract from Plan Artifact 2.2; ROUND2 requirements | digest | **W=4** scenario authors (cheap-eligible: deterministic asserts) | T1/T1.5 authors + T2 assembler | M / L |
| **3.2** | G3 moat bench vs SQLite+sqlite-vec+CTE (+libSQL DiskANN) with the **pre-registered STOP numbers** (kill < 20% p50 saving AND < 2× round-trip cut; proceed ≥ 2× and ≥ 30%) | C-2 | `benchmark/**` | BENCH-SPEC; baseline harness; correctness suite | digest | W=2 (ours + baseline) | T2 + T3 verdict write-up | L / M |
| **3.3** | **DECISION GATE (USER):** fund/kill deferred GNSE backlog from bench evidence + first-10-installs signal | C-0 (decision doc) | `docs/` | 3.2 report | digest | — | USER | S / — |

## 4. Execution DAG + ready frontier

```
Frontier @ day 1 (parallel — disjoint locks):
  WP-0.1 (docs/adr)      WP-1.1 (src/lib.rs 🔒)      WP-3.1-scaffold (tests/)

WP-1.1 ──merge──▶ [WP-0.2 optional, global 🔒, alone] ──▶ WP-1.2 ──▶ WP-1.3 ─┐
WP-0.1 (ADR accepted) ──required-by──▶ WP-1.3 (retention policy)             │
WP-1.2 ──▶ WP-2.1 ──▶ WP-2.2 ──▶ WP-2.3(may slot earlier after 1.1 if idle)  │
WP-3.1 (parallel, no engine lock) ──▶ WP-3.2 ◀── WP-2.2 ──────────────────────┘
WP-3.2 ──▶ WP-3.3 (USER decision gate)
```

Sprint mapping (1 sprint = 1 branch per runbook): Sprint 0 = {0.1, 1.1} · Sprint 1 = {0.2?, 1.2} · Sprint 2 = {1.3, 2.1, 3.1} · Sprint 3 = {2.2, 2.3} · Sprint 4 = {3.2, 3.3}.

GoVibe hierarchy: this plan = **H4** theme; each phase = **H3** epic; each WP = **H2/H1**; each PR = **H0**. Labels per runbook (`plan-review` → `plan-approved` → `file-lock` → done); `blocked` red label on any packet whose verify gate fails twice → rework loop back to Planner (not silent retry).

## 5. Verification & acceptance summary (3 nested levels, per tiered-swarm)

1. **Epic DoD (human, WP-3.3):** tx-time probe passes; bench verdict written with STOP numbers honored; no invariant regressions; USER decision on backlog recorded.
2. **Phase gates (machine + Lead):** as listed per phase above — these are the routing interlocks; no unverified cheap output crosses them.
3. **Per-packet `verify_command`:** each WP's command IS the TDD RED test; a packet without one cannot be routed below T2.

## 6. Budget & risk posture

- **T3 (Opus/Fable) spend concentrates on:** WP-0.1 authoring, 1.2 format review, phase-gate reviews, 3.2 verdict. Everything else routes T2 or below.
- **Cheap tiers (T1/T1.5/T0) allowed only on:** test-scenario authoring (3.1), parity tables (2.2), bench runners (1.3/3.2), triage — all carry deterministic `verify_command`s.
- **Never cheap:** anything under `src/lib.rs` durability paths (1.1/1.2/1.3) — high-risk domain override.
- **Highest-risk packets = 1.1/1.2/1.3** (on-disk + recovery): each requires Verifier + Auditor sign-off and a crash-matrix Verification Artifact before merge.

## 7. Rework / escalation loop

Verify-gate fail → Verifier files Verification Artifact with re-runnable evidence → packet re-enters Ready Frontier as rework at **one tier higher** (tiered-swarm escalation T1→T1.5→T2→T3); two consecutive fails → `blocked` + Planner re-plans the packet (scope or approach), Lead arbitrates; scope disputes → USER (runbook §7).

## 8. Deferred backlog (evidence-gated — do NOT schedule)

| Item | Trigger to activate |
|---|---|
| Native segment stores + 16KiB page cache (GNSE T2/T3) | a real consumer (NotiKeeper/MSP) exceeds measured RAM budget, or mobile GA requires paging below 300MB–2GB envelope |
| Epoch-segmented HNSW / vector time-travel | a consumer demands "vector search as of commit N" (none does today); until then: version-stamp vector metadata + brute-force rerank for historical |
| Full CommitFrame prev-hash chain across CRDT sync | History ADR follow-up after 1.2 stabilizes; requires sync-wire redesign decision |
| SQLite property demotion (GNSE T6) | only after segment property store exists; CRM tier actively wants the relational projection — keep it |

---

## CHANGELOG

| Version | Date | Summary |
|---|---|---|
| draft-1 | 2026-08-17 | Initial multi-agent remediation roadmap mapping GNSE thin slice onto GoVibe C/H/R/D/W/Budget/Risk orchestration |
