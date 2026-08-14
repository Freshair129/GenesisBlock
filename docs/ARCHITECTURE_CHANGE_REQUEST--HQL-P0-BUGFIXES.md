---
status: superseded
superseded_by: adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md
updated: 2026-08-14
---

# ARCHITECTURE_CHANGE_REQUEST — Fold HQL P0 bugfixes into Wave 0 (pre-publish)

**Status:** implemented historical amendment; superseded for future query planning · **Requested:** 2026-07-07 · **Requester:** Boss (CEO)
**Amends:** [MASTER_PLAN.md](MASTER_PLAN.md) §0 (frozen constraints), §1 (roadmap), §2 (dependency graph); [PHASE_6_REVIEW.md](PHASE_6_REVIEW.md); [queue/IMPLEMENTATION_QUEUE.json](../queue/IMPLEMENTATION_QUEUE.json).
**Preserves:** [ADR--ENGINE-WEDGE-FIRST](adr/ADR--ENGINE-WEDGE-FIRST.md) — the wedge-first discipline is not reversed; only its pre-publish scope is widened by ≤3 tasks to avoid shipping known defects.

> **Truth sync (2026-08-14):** HQL P0 landed on `main` and is now the compatibility baseline.
> The pre-merge baseline window described below is historical and cannot be scheduled again. Future
> primary-query work is governed by the Typed Query IR ADR/spec; HQL P1/P2/P3 remain deferred.

---

## 1. Reason

The MasterPlan's frozen constraints list *"HQL grammar semantics"* as untouchable, so
`docs/PLAN--HQL-REFINEMENT.md` `P0` work (grammar + `execute_hql` changes, uncommitted in the
working tree) formally needs an ACR. Beyond process compliance, three business reasons make
this a Wave-0 mini rather than a post-gate item:

1. **Publishing engine v0.2.0 with these defects present is dishonest.** `P0` items are
   defects, not enhancements:
   - `SEARCH`/`MATCH … SIMILAR` compute a fuzzy-resolved target and **throw it away** — the
     documented feature does not work.
   - Hybrid candidate pool is **hardcoded `K=10`** — grammar has no `K` clause on the hybrid
     form; users cannot widen the pool.
   - `ef_search` and `oversample` — engine capabilities the P32 recall fix depends on — are
     **unreachable from HQL** (grammar has no `EF` / `OVERSAMPLE` tokens).
   - Traversal `DIRECTION` and multi-rel alternation exist internally but have no grammar.
   Shipping these under GATE-DEMAND-1's "does anyone install this" question yields a **false
   negative signal** — a stranger's smoke test hits documented-but-broken behavior and moves on.

2. **Baseline preservation hazard is time-critical.** Genesis PROPOSAL (Path 1, Stage 0)
   flagged: *"the pre-P0 v1 baseline must be captured on clean main BEFORE P0 merges — merging
   first destroys the before/after chain."* Once P0 lands on `main`, the v1 baseline for the
   BENCH-SPEC's HQL-vs-Cypher expressiveness test cannot be reconstructed. The window closes on
   the next merge to main, not on gate day.

3. **Scope discipline preserved.** Only `P0` (correctness + exposure) is folded in.
   `P1` (variable-length paths), `P2` (OR / label-index), `P3` (text-query ADR) **remain
   deferred behind GATE-DEMAND-1**, exactly as the ADR requires. If the demand signal fails,
   none of P1–P3 gets built. This ACR does not reverse the wedge — it widens W0 by one task
   family so what we do publish is honest.

## 2. Impact

**Frozen-constraint deltas** (MasterPlan §0):
- **HQL grammar semantics** — narrowly unfrozen for `P0` only, per PLAN--HQL-REFINEMENT §P0 and
  the `Track boundary (2026-07-05)` note in that PLAN and in SPEC--HQL-V2. Grammar additions
  are strictly additive: new tokens (`EF`, `OVERSAMPLE`, `DIRECTION`), new rules
  (`similar_clause`, `rel_type` alternation, `direction_spec`), and new `search-by-node`
  semantics when the caller omits `SIMILAR TO [...]`. Existing queries continue to parse
  unchanged. P1–P3 semantics remain frozen.
- All other frozen constraints (Storage core methods, public NAPI/REST/FFI signatures, on-disk
  / WAL format) **unchanged**.

**Wave delta** (MasterPlan §1):
- Add **W0 Pre-publish (P0 native track)** *before* W1.
- W1's exit criterion unchanged: `npm install` on a clean machine → smoke passes.
- Gates unchanged: GATE-DEMAND-1 still blocks W2–W4.

**Queue delta**: 3 new tasks (TASK-0000a/b/c). Task IDs are stable and never re-used per §12.2.

## 3. Affected modules

- `src/query/hql.pest` — grammar additions
- `src/query/ast.rs` — AST plumbing for new clauses
- `src/lib.rs` — `execute_hql` uses resolved target + `ef` / `oversample` / direction plumbing
- `tests/hql*.rs` — coverage of new grammar and search-by-node semantics
- `docs/PLAN--HQL-REFINEMENT.md`, `docs/SPEC--HQL-V2.md` — already carry the Track-boundary note
- `benches/` — bench script(s) to capture v1 / post-fix baseline (TASK-0000a, TASK-0000c)

Public NAPI/REST/FFI signatures **do not change** — HQL is a query-language surface, not a wire
API.

## 4. Migration plan

**Order-of-operations (owner-authority sequencing — the hazard is real):**

1. **TASK-0000a — Capture v1 pre-P0 baseline on clean `main`.** Run the existing HQL bench
   scripts against `main` HEAD (before any P0 code lands). Commit the raw output under
   `benches/baselines/hql-v1/`. Non-negotiable ordering: this is the gate before TASK-0000b.
2. **TASK-0000b — Merge P0 defect fixes.** Commit the P0 changes to `src/query/*`,
   `src/lib.rs`, and `tests/hql*.rs`. Existing tests remain green; new tests cover new
   grammar; PLAN/SPEC boundary-note edits ride along.
3. **TASK-0000c — Capture post-fix baseline.** Re-run the same bench harness; commit output
   under `benches/baselines/hql-v2-p0/`. Diff summary lands in `CHANGELOG.md` as a two-line
   note.
4. **W1 proceeds as planned.** With P0 landed, `npm install` publishes an engine that honors
   its documented grammar.

**Rollback:** if TASK-0000c shows a p50 or recall regression >5% vs the v1 baseline on any
existing bench that measured pre-P0 behavior *at equal semantic input*, revert the P0 patch
and re-open the ACR with the failing measurement. Semantic changes (search-by-node with the
same user intent) are not measured as regressions.

## 5. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **Baseline captured on a dirty tree (P0 already applied)** | Med | High | ACR §4 step 1 is mandatory; verify `git status` clean at capture time |
| P0 changes leak into P1/P2 semantics under time pressure | Low | Med | Track-boundary notes in PLAN & SPEC (2026-07-05) enforce separation |
| Scope creep: someone appends "just one P1 task" | Med | Med | ACR §1 scope is explicit; a P1 fold requires a new ACR |
| Post-fix bench shows regression | Low | Med | Rollback path in §4 |
| Time cost delays W1 | Low | Low | Sized ≤3 working days; publishing was gated on packaging tasks (TASK-0001) that require CI validation anyway |

## 6. Alternatives (rejected)

- **A. Ship engine v0.2.0 as-is with P0 defects present.** Rejected: GATE-DEMAND-1 demands a
  clean demand signal; broken documented features poison the signal.
- **B. Defer P0 entirely behind GATE-DEMAND-1 (fold with P1–P3).** Rejected: the pre-P0
  baseline window closes on the next `main` merge; deferral throws away the baseline.
- **C. Fold in P0 + P1 + P2 + P3 (full HQL refinement, 22 tasks).** Rejected: reverses the
  wedge discipline; the ADR's whole thesis is "publish cheap, measure demand, then invest."
  If chosen, a *new* ADR superseding ADR--ENGINE-WEDGE-FIRST is required first.

## 7. Approval

Implemented and merged before this truth-sync. The old W0 queue artifacts are retained as historical
planning evidence but are superseded by `MASTER_PLAN.md` v0.2.0. This status does not claim that the
original pre-P0 benchmark baseline was captured unless a committed artifact independently proves it.
