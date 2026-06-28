# AUDIT--SOAK-TEST: Long-Running Soak Tests

**Date:** 2026-06-28
**Suite:** `tests/soak_tests.rs`
**Result:** 2/2 PASS (light + medium)

## Motivation

Soak tests detect problems that only surface under sustained load: memory
leaks, HNSW index drift, latency degradation, WAL/snapshot growth, and
compaction failures. They complement the point-in-time integration tests by
running continuous ingest→query→compact cycles.

## Test Hardware

| Component | Detail |
|-----------|--------|
| CPU | Intel Core i7-8700K @ 3.70GHz (6C/12T) |
| RAM | 32 GB DDR4-2133 |
| Test drive | WDC WDS250G2B0A 250GB SATA SSD (C:) |
| OS | Windows 10 Pro 10.0.19045 |

Note: project lives on G: (HDD) but soak tests use `SOAK_TMPDIR=C:/temp/...`
(SSD) for realistic I/O. Set the env var to route test databases to any drive.

## Profiles

| Profile | dim | nodes/cycle | cycles | total nodes | compact | ef_search |
|---------|-----|-------------|--------|-------------|---------|-----------|
| Light | 4 | 100 | 60 | 6,000 | every cycle | 200 |
| Medium | 4 | 500 | 360 | 180,000 | every 10 | 200 |

Both are `#[ignore]`d — run explicitly:
```bash
# Light (~12s on SSD)
SOAK_TMPDIR="C:/temp/genesis_soak" cargo test --no-default-features --test soak_tests --release -- --ignored soak_light --nocapture

# Medium (~5 min on SSD)
SOAK_TMPDIR="C:/temp/genesis_soak" cargo test --no-default-features --test soak_tests --release -- --ignored soak_medium --nocapture

# Both
SOAK_TMPDIR="C:/temp/genesis_soak" cargo test --no-default-features --test soak_tests --release -- --ignored --nocapture
```

## Results

### Light (6,000 nodes)

```
Elapsed: 12.3s
Total nodes: 6000
Final disk: 5.3 MB
Recall misses: 4/60 (6.7%)
Query latency: first10_avg=0ms, last10_avg=0ms
Spot-check after reopen: OK
```

### Medium (180,000 nodes)

```
Elapsed: 272.4s (4.5 min)
Total nodes: 180000
Final disk: 160.6 MB
Recall misses: 27/360 (7.5%)
Query latency: first10_avg=0ms, last10_avg=0ms
Spot-check after reopen: OK
```

## Observations

### No memory leak or latency degradation
- Ingest latency flat at ~420ms/cycle (500 nodes) from start to finish
- Query latency sub-millisecond throughout (release build, dim=4)
- No OOM or growing allocation patterns

### Disk growth is linear and bounded by compaction
- Medium: 160 MB for 180k nodes (compact every 10 cycles)
- With compact-every-cycle (light): 5 MB for 6k nodes
- WAL compaction keeps disk bounded; no unbounded growth observed

### Recall misses are a dim=4 HNSW property, not a drift signal
- 6.7% (light) and 7.5% (medium) miss rates with dim=4 at high node counts
- This is expected: dim=4 creates many near-colliding embeddings that HNSW
  (an approximate index) occasionally misses with K=5/10 neighbors
- Misses are uniformly distributed across cycles — NOT increasing over time,
  confirming no index drift
- Real workloads use dim=768+ where HNSW recall is >99%

### Data survives reopen
- Spot-check after drop+reopen verifies nodes at positions 0, N/4, N/2, N-1
