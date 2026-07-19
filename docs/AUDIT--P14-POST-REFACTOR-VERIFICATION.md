---
proposed_id: AUDIT--P14-POST-REFACTOR-VERIFICATION
type: audit
status: historical
aliases:
  - AUDIT
  - P14
tier: process
cluster: implementation_flow
role: "Post-refactor benchmark verification"
phase: 14
audited_at: 2026-06-21
proposed_by: agent
related:
  - CR--EDGE-ENDPOINTS-STRING-AND-EMBEDDING-DEDUP
  - INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS
  - AUDIT--P12-SCIENTIFIC-VERIFICATION-REPORT
  - AUDIT--P13-GROUP-COMMIT-REPORT
commits:
  - b5e9771
  - 75d560e
---

# AUDIT — P14 Post-Refactor Verification

## 1. Purpose

Re-run the P7–P13 benchmark binaries after the edge-endpoint revert (b5e9771)
and the in-memory embedding dedup (75d560e) to (a) confirm the suite is
runnable again — it was uncompilable — and (b) measure the improvement, with
honest separation of what is comparable to the historical audits and what is
not.

## 2. ⚠️ Environment caveat — storage medium differs

**The historical P7–P13 audits ran on NVMe/SSD** (the reports explicitly cite
"NVMe `sync_all()`"). **This machine runs the project from `G:` which is a
7200 RPM HDD** (`WDC WD10EZEX-00RKKA0`). `C:` is an SSD (`WDC WDS250G2B0A`).

Any **fsync-bound write throughput** number is therefore **not comparable** to
the historical baselines — it is dominated by per-commit disk latency
(HDD ~10–40 ms seek+rotation vs SSD/NVMe ~0.1–1 ms), not by engine code.

**Proof (same binary, only the WAL target differs):**

| Bench | `G:` (HDD) | `C:` (SSD, via working dir) | Storage factor |
|---|---|---|---|
| `snb-bulk-ingestion` (5k, single-thread, per-op fsync) | 24.34 nodes/s | **1,128.68 nodes/s** | **~46×** |
| `industrial-audit` (10k, single-thread) | 22.51 nodes/s | **943.79 nodes/s** | **~42×** |

So the low `G:` write rates are the HDD, not a regression. The refactor did not
touch the WAL/fsync path.

> Historical note: `AUDIT--P12-SCIENTIFIC-VERIFICATION-REPORT` already retracted
> the pre-P12 "400k QPS / 20k TPS" figures as measurement artifacts (stubbed
> reads, no durability). The honest durable baseline is P13 (834 TPS, 12-thread,
> NVMe, group commit). Single-threaded per-op fsync does not benefit from group
> commit (nothing to batch), so single-thread rates are expected to be modest on
> any disk and HDD-bound here.

## 3. Memory — the real, comparable improvement

RAM is independent of disk and is the headline win of this refactor (P-B drops
the redundant in-memory f64 embedding; arena + HNSW remain the f32 source of
truth).

| Workload | Before P-B | After P-B | Δ |
|---|---|---|---|
| `scientific-audit` 5k nodes × 1536-dim (net RSS) | 147 MB | **82 MB** | **−44%** |
| `shadow-sync-stress` 10k nodes × 1536-dim, 12 writers / 4 readers, JSON props (peak RSS) | — | **213 MB** | — |

The 10k × 1536 concurrent run peaking at **213 MB** is the standout: the P11
report logged ~**31.96 GB** for a 10k-node mixed-workload stress (a different
harness that also accumulated 50k+ result sets, so not apples-to-apples), and
the P12 report logged **15.89 GB** at 32k nodes on the old Mark VII engine. The
current engine is in the low hundreds of MB at the same node scale.

## 4. Query / in-memory latency — comparable

CPU- and memory-bound, unaffected by the HDD.

| Bench | Metric | P14 result | Historical |
|---|---|---|---|
| `hql-query-stress` (1k × 1536) | avg HQL query latency | **10.49 µs** | P10 mean 27 µs |
| `shadow-sync-stress` (under concurrent load) | P95 query latency | **432 µs** | — |
| `ldbc_lite` (1k / 5 fan-out) | 1-hop / 2-hop / 3-hop traversal (median) | **8.37 / 78.28 / 527.85 µs** | P7: 12.79 / 81.03 / 522.07 µs |

Traversal is on par with (1-hop faster than) the P7 baseline — confirming the
edge revert's extra `get_u32` string→u32 resolution per hop adds no measurable
cost on the traversal hot path.

## 5. Correctness & durability — restored

- **Build/run:** all six audit binaries + the criterion bench compile and run
  again (every one failed to compile before the revert).
- **Durability path verified:** `shadow-sync-stress` performed graceful
  shutdown → snapshot save → **instant load 84 ms** → WAL replay →
  `VERIFICATION: Node 'Note-9999' found. WAL Replay SUCCESS.` This exercises the
  snapshot load path and confirms the HNSW-rehydrate-on-both-paths fix (b5e9771)
  works end to end.

## 6. Conclusion

- **No throughput regression** from the refactor; the apparent slowdown is the
  HDD WAL target (proven ~42–46× recovered on SSD with the same binary).
- **RAM materially reduced** (−44% on the f64 dedup; 10k × 1536 concurrent at
  ~213 MB).
- **Suite restored** to a runnable, durable, green state.
- For representative throughput numbers, run on `C:`/NVMe; `G:` (HDD) is a
  development convenience, not a perf target.

## 7. Raw results (this run, 2026-06-21)

```
snb-ingestion        1000 persons              74.29 s            (G:/HDD)
snb-bulk-ingestion   5000 nodes                24.34 nodes/s      (G:/HDD)
                                               1,128.68 nodes/s   (C:/SSD)
industrial-audit     10000 nodes               22.51 nodes/s      (G:/HDD)
                                               943.79 nodes/s     (C:/SSD)
hql-query-stress     1k×1536, 100 iters        10.49 µs/query
shadow-sync-stress   10k×1536, 12w/4r          118.34 TPS, P95 432 µs, peak RSS 213 MB, total 84.5 s (G:/HDD)
scientific-audit     5k×1536                    net RSS 82 MB (was 147 MB pre-P-B)
ldbc_lite (criterion) 1k/5 fan-out              1-hop 8.37 µs / 2-hop 78.28 µs / 3-hop 527.85 µs
```

## 8. Hardware

- CPU/RAM host as configured; project disk `G:` = `WDC WD10EZEX-00RKKA0`
  (7200 RPM HDD); `C:` = `WDC WDS250G2B0A` (SSD), used for the storage-isolation
  runs in §2.
- hnsw_rs 0.3.4, `init_hnsw(M=16, ef_construction=200)`; vector dim 1536 (f64
  input → f32 arena/index).
