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

## Follow-ups (DONE 2026-08-20 — see [REPORT--MOAT-FOLLOWUPS](REPORT--MOAT-FOLLOWUPS.md))

Per the verdict's recommendation, both were scheduled as the next bench work, and the
moat claim did not ship publicly until they landed. **Both are now measured, and
neither caveat was hiding a weakness:**

1. ~~**libSQL DiskANN baseline row** in the moat bench — attacks Q4 (the brute scan);
   expected to narrow the single-axis control, not the fused shapes or the correctness gate.~~
   **MEASURED.** The prediction held on the fused shape (engine still 45.7×/46.9× ahead)
   but understated the single-axis result: DiskANN beat the brute scan by only
   1.21×/1.88×, leaving the engine ~12–13× ahead on the vector axis it indexes — while
   costing 8.5×–11.8× the engine's ingest. Measured at N=11,266 (a 100k libSQL run is
   hours of ingest); do not quote it at 100k.
2. ~~**Real-corpus (bge-m3) moat run** — replaces the synthetic-vectors caveat with a
   measured real-embedding result.~~ **MEASURED, and the caveat was conservative.**
   At matched N, real embeddings *raise* every vector-touching ratio (q1 52.3× → 67.2×,
   q4 16.2× → 22.8×) because clustered vectors navigate the HNSW graph better while the
   baseline's O(N) scan is distribution-blind. The graph-only control moves −4% (noise),
   which is what makes the causal read credible.

Public positioning is therefore unblocked, with the guidance in the report: quote the
100k synthetic figures as the conservative headline, always with N stated, and label the
libSQL numbers as 11k-scale.

## Consequences

- The GNSE remediation line (WP-0.1 → WP-3.2, PRs #100–#114) is **closed**. Epic DoD
  (plan §5.1) is met: tx-time probe passes, bench verdict written with STOP numbers
  honored, no invariant regressions, USER decision recorded (this doc).
- Next engineering work item: spec + plan for epoch-segmented HNSW / vector time-travel
  (design doc first, per DDD workflow), alongside the two bench follow-ups.
- The three unfunded items remain in plan §8 under their original activation triggers —
  this decision does not kill them, it declines to schedule them.
