---
proposed_id: AUDIT--P24-P25-GOVERNANCE-AND-KIMPACT-COST
type: audit
status: complete
aliases:
  - AUDIT
  - P24
  - P25
tier: process
cluster: implementation_flow
role: "Governance guard cost + K-Impact full-vs-incremental (O(V_affected) proof)"
phase: 24
audited_at: 2026-06-21
proposed_by: agent
related:
  - SPEC--KIMPACT-AND-INFERENCE
  - SPEC--AXIOMATIC-GUARDS
  - AUDIT--P22-GRAPH-TRAVERSAL
---

# AUDIT — P24 Governance cost & P25 K-Impact cost

`benches/gov_kimpact_bench.rs` (`[[bin]] gov-kimpact-bench`), C: SSD.

## P24 — Governance guard cost (off vs on)

`validate_governance` = `Tier::from_labels(labels)` + a MASTER/system check.
Microbenchmarked at 5M iterations:

| Path | ns/op |
|---|---|
| baseline (no guard) | 0.46 |
| guard ON, USER labels (pass, 3 labels) | 524.6 → **overhead ≈ 0.52 µs** |
| guard ON, MASTER label (reject, 1 label) | 104.6 |

**Reading:** the guard costs **~0.5 µs/op**. Against a durable `add_node`
(~0.5 ms batched … ~2.6 ms per-op, WAL fsync) that is **< 0.1 % overhead** —
effectively free on the write path. Governance is not a throughput concern.

**Optimization found:** the pass-path (USER, 3 labels) is ~5× the reject-path
(MASTER, 1 label) because `Tier::from_labels` allocates per label (case
folding). Switching to `eq_ignore_ascii_case` (no allocation) would cut the
guard ~10×. Not urgent given it's already <0.1 % of a write.

## P25 — K-Impact: full vs incremental recompute

Claim (SPEC--KIMPACT): localized updates are **O(V_affected + E_affected)**, not
a full O(V) pass. Measured `refresh_impacts(None)` (full) vs
`refresh_impacts(Some([one_id]))` (incremental, avg of 2000 single-node updates)
on random graphs (fanout 8):

| Nodes | Full recompute | Incremental(1) | Speedup |
|---|---|---|---|
| 10,000 | 9.00 ms | 0.916 µs | 9,827× |
| 100,000 | 104.0 ms | 1.480 µs | 70,287× |
| 500,000 | 664.2 ms | 1.668 µs | 398,105× |

**Reading — claim proven:**
- **Full** scales with N (9 → 104 → 664 ms ≈ O(V)).
- **Incremental(1)** stays ~flat (0.92 → 1.48 → 1.67 µs) across a 50× node
  increase — i.e. **O(V_affected)**, independent of total graph size.
- Speedup therefore grows with N (up to ~400,000× at 500k).

`compute_impact` per node is O(1) (incoming-edge count via `in_idx`, tier score,
SC), so a k-affected update is O(k) — the localized-BFS impact model behaves as
specified. The whitepaper's incremental-update claim now has direct evidence.

## Status vs program

P24, P25 done. Remaining: **P23** Neo4j (and ideally Kuzu) head-to-head on the
same graph (traversal latency / memory / ingest) — Docker is now available.

Reproduce:
```
GB_VBENCH=<dir> GB_KIMPACT_SIZES=10000,100000,500000 cargo run --release --bin gov-kimpact-bench
```
