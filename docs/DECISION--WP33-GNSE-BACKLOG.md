---
status: current
---

# DECISION — WP-3.3 GNSE Deferred-Backlog Gate (USER)

> **Status:** decided · **Decided by:** USER · **Date:** 2026-08-19
> **Plan:** `PLAN--GNSE-REMEDIATION-MULTIAGENT.md` §3 (WP-3.3, C-0) / §8 (deferred backlog)
> **Evidence consumed:** `REPORT--G3-MOAT-VERDICT.md` (WP-3.2, verdict **PROCEED** —
> 114.9–187.9× fused p50 vs single-SQLite-file assembly at 100k×1024; baseline fails
> 2/5 bitemporal correctness scenarios structurally).

## Decision

**Fund selectively.** Of the four §8 deferred-backlog items, exactly one is activated:

| §8 item | Decision | Rationale |
|---|---|---|
| **Epoch-segmented HNSW / vector time-travel** | **FUNDED** | The one item the read-side moat evidence actually gates. Also closes the disclosed WP-2.2 limitation pinned by the `#[ignore]`d WP-3.1 RED test (`tests/bitemporal_matrix_wp31_tests.rs`: `tx_as_of` cannot resurrect retracted nodes) — funding it turns that test green. |
| Native segment stores + 16KiB page cache | stays deferred | Own trigger (consumer RAM-budget breach / mobile GA paging) has not fired. |
| CommitFrame prev-hash chain across CRDT sync | stays deferred | Requires a sync-wire redesign decision; not touched by the bench evidence. |
| SQLite property demotion | stays deferred | Prerequisite (segment property store) does not exist; CRM tier actively wants the relational projection. |

## Evidence honesty note

The gate spec reads "bench evidence **+ first-10-installs signal**". The bench half is in
(PROCEED, reproduced, verify-gated); the install half is **not** — NotiKeeper is the only
real embedded consumer to date. The selective scope is sized to the evidence that exists:
the full-backlog option was explicitly declined as running ahead of the install signal.

## Follow-ups (scheduled — prerequisites for public positioning)

Per the verdict's recommendation, both are scheduled as the next bench work, and the
moat claim does not ship publicly until they land:

1. **libSQL DiskANN baseline row** in the moat bench — attacks Q4 (the brute scan);
   expected to narrow the single-axis control, not the fused shapes or the correctness gate.
2. **Real-corpus (bge-m3) moat run** — replaces the synthetic-vectors caveat with a
   measured real-embedding result.

## Consequences

- The GNSE remediation line (WP-0.1 → WP-3.2, PRs #100–#114) is **closed**. Epic DoD
  (plan §5.1) is met: tx-time probe passes, bench verdict written with STOP numbers
  honored, no invariant regressions, USER decision recorded (this doc).
- Next engineering work item: spec + plan for epoch-segmented HNSW / vector time-travel
  (design doc first, per DDD workflow), alongside the two bench follow-ups.
- The three unfunded items remain in plan §8 under their original activation triggers —
  this decision does not kill them, it declines to schedule them.
