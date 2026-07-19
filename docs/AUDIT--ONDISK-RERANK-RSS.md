---
status: historical
---

# AUDIT — On-disk rerank sidecar: resident-RAM restoration (P0-T9)

**Change under audit:** VECTOR-QUANTIZATION P0 — the exact-f32 rerank sidecar moved
from a resident `Option<RwLock<Vec<f32>>>` to a file-backed `Option<RwLock<SidecarReader>>`
(positioned `seek_read`/`read_at` over `fvec_<name>.bin`, fronted by a bounded LRU).
Branch `feat/vq-p0-ondisk-sidecar` (commits `fa03a47`, `10f2e63`, `96f5214`).

**Question T9 answers:** does moving the sidecar off-RAM restore the quantization RAM
win that the resident `Vec<f32>` cancelled (RCA §"Per-vector resident RAM")?

---

## 1. Structural result (certain by construction)

The RCA established that for a BQ rerank-on collection the resident f32 sidecar is
**94.1%** of per-vector resident bytes (6144 / 6528 B at 1536-dim). That `Vec<f32>` no
longer exists: `f32_sidecar` is now a `SidecarReader` holding a `File` + a bounded LRU
(`SIDECAR_CACHE_ROWS = 4096`). The exact vectors live in `fvec_<name>.bin` on disk;
resident RAM holds only the LRU working set.

Per-vector resident RAM, **after** the move (mirrors RCA table 106–112; sidecar column
→ 0 because it is no longer resident):

| Config          | arena (B) | HNSW copy (B) | sidecar resident (B) | resident/vec (B) | reduction vs f32 | before (RCA) |
|-----------------|----------:|--------------:|---------------------:|-----------------:|-----------------:|-------------:|
| None (f32)      | 6144      | 6144          | —                    | **12288**        | 1.00×            | 12288        |
| SQ8, rerank off | 1536      | 1536          | —                    | **3072**         | 4.00×            | 3072         |
| SQ8, rerank on  | 1536      | 1536          | **0** (was 6144)     | **3072**         | **4.00×**        | 9216 (1.33×) |
| BQ, rerank off  | 192       | 192           | —                    | **384**          | 32.0×            | 384          |
| BQ, rerank on   | 192       | 192           | **0** (was 6144)     | **384**          | **32.0×**        | 6528 (1.88×) |

**Rerank-on now costs the same resident RAM as rerank-off.** BQ rerank-on is restored
from **1.88× → 32.0×**; SQ8 rerank-on from **1.33× → 4.00×**.

### The new resident term is O(1) in N, not O(N)

The old sidecar scaled with corpus size: `N × dim × 4` bytes resident. The new term is
the bounded LRU, capped at **`SIDECAR_CACHE_ROWS × dim × 4`** *regardless of N* —
≈ **24 MiB** at 1536-dim (≈ 16 MiB at 1024-dim), per collection. This is the load-bearing
property: at scale the sidecar contributes a fixed cache ceiling, not a per-vector tax.

### Projection @500k, 1536-dim (BQ rerank-on)

| Term                    | before (resident Vec) | after (on-disk + LRU) |
|-------------------------|----------------------:|----------------------:|
| Sidecar resident RAM    | 500 000 × 6144 = **2.86 GiB** | ≤ **24 MiB** (LRU cap) |
| Sidecar on disk (`fvec`)| 2.86 GiB (snapshot only) | 2.86 GiB (live file) |
| Collection resident/vec | 6528 B → 3.06 GiB total | 384 B → **183 MiB** total |

The exact vectors are not lost — they move from RAM to `fvec_<name>.bin`, matching
Qdrant's "compressed in RAM, original on disk, rescore top-N" model the ADR targeted.

## 2. Recall is unaffected

The rerank arithmetic is byte-identical; only the row *source* changed (RAM slice →
positioned read of the same `d_id*dim*4` offset). The behavioral oracle proves exactness:
`rerank_tests` (6), `sidecar_ondisk_tests` (6, incl. BQ/SQ8 parity + reopen + compaction),
`sidecar_migration_tests` (2), `sidecar_reader_tests` (2) — all green, unedited. The
MARK XV recall figures (SQ8+rerank 0.9875, BQ+rerank 0.9655, BQ-alone 0.6845 unusable;
see RCA / `project_mark_xv_rss500k`) therefore carry over unchanged — same rerank, off-RAM.

## 3. Empirical RSS sweep (reproducible; run on the dev host)

The structural result above is exact. The empirical @500k confirmation is a heavy
release-harness run (LTO build + multi-GB corpus) and is **not run in this PR** — there is
currently no 500k corpus on the box (only a 1M corpus at `C:\Users\freshair\gb_vbench_1m`),
and the sweep is disruptive on an in-use machine. To reproduce on the dev host:

```powershell
# 1. generate a 500k corpus (dim 1536) into C:\Users\freshair\gb_vbench_500k\corpus.f32
#    (same generator MARK XV used; see benches/scripts/rss_probe.md)
# 2. sweep {none,sq8,bq} x {rerank 0/1}, recording engine RSS + on-disk gdb size:
powershell -File benches/scripts/rss_matrix.ps1
#    -> C:\Users\freshair\rss_disk_matrix.txt  (one line per config: RSS + disk_mb)
```

**Expected (matches §1):** for the `bq rerank=1` row, engine RSS drops from the resident
baseline toward the `bq rerank=0` RSS (the ~24 MiB LRU is in the noise at 500k), while the
on-disk `gdb` size *grows* by the sidecar's `N×dim×4` — RAM win, disk cost, as designed.
Paste the `bq rerank=0` vs `bq rerank=1` RSS delta here once run to close the empirical gate.

---

**Status:** structural/analytical proof complete and certain; correctness oracle green.
Empirical 500k RSS delta pending a dev-host harness run (command above). This is the only
outstanding P0 item; all code, tests, and docs (T0–T8, T10) are merged on the branch.
