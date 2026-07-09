# Phase 6 Review — Handoff & Task Decomposition

**Status:** awaiting_approval (2026-07-07)

## What was completed
- **18 tasks** decomposed across 4 waves per RWANG §12.2 template.
- **Machine SSOT:** `queue/IMPLEMENTATION_QUEUE.json`, `queue/PROJECT_GRAPH.json`.
- **Human view:** `docs/33_TASK_BREAKDOWN.md`, `docs/36_TASK_EXECUTION_ORDER.md`.
- Every task carries: id, wave, category, deps, priority, capability, complexity, context,
  `LOCAL_SAFE`/`CLOUD_REQUIRED`, verification, ready-flag, and (where applicable) `gated_by`.
- **GATE-DEMAND-1** encoded as a first-class object in the queue JSON (blocks W2/W3/W4).

## Local-dispatch breakdown
- 18 tasks total: **12 LOCAL_SAFE (67%)** / 6 CLOUD_REQUIRED (33%).
- **Ready-now for local pickup (deps=[], ready=true, LOCAL_SAFE): TASK-0002, TASK-0003, TASK-0009, TASK-0011, TASK-0012, TASK-0014.**
- CLOUD_REQUIRED tasks are the ones that legitimately require secrets/CI/publish rights or public interface additions/architecture decisions — not artificially locked.

## Frozen constraints (invariants for every task)
Do NOT change: Storage core methods, public NAPI/REST/FFI signatures, on-disk/WAL format, HQL
grammar semantics. Any such change requires an `ARCHITECTURE_CHANGE_REQUEST`.

## Open questions / risks
1. **`NPM_TOKEN` and release matrix never validated end-to-end** — TASK-0001 verifies this first;
   it is the top-of-queue risk.
2. **Graphiti `GraphDriver` contract unknown** — TASK-0016 gates TASK-0017/0018 and can flip the
   W4 plan from adapter to translator (upgrading its complexity from M to L).
3. **Private MSP/GKS runtime repo** — the on-disk audit only spot-checked D:. Owner may confirm;
   if a real runtime exists elsewhere, this could re-open a platform path (would need an
   `ARCHITECTURE_CHANGE_REQUEST`).
4. **LYRA ROUND4** was pending at ADR write time. If it lands and materially challenges the
   engine-wedge decision, revisit via ADR amendment.

## Ready for Phase 7
Yes — on owner approval of this Phase 6 review, the queue becomes dispatchable. Wave 1 kicks
off with TASK-0001 (cloud/human) in parallel with TASK-0002/0003 (local).
