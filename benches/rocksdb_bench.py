"""
MARK XV — RocksDB + graph head-to-head (embedded KV store + adjacency lists).
RocksDB is an embedded LSM key-value store; the idiomatic "graph layer" stores
each node's adjacency list as one KV entry (key=node id, value=neighbor ids) and
traverses by BFS over point lookups. Same topology as P22/P26/P28 (N nodes,
fanout-8 random), depths {1,3,6}. Mirrors kuzu_bench.py / duckdb_bench.py so the
GenesisBlockDB side (graph_results_{N}.json) is directly comparable.
"""
import os, json, time, random, shutil
import numpy as np
from rocksdict import Rdict, Options

try:
    import psutil
    def rss_mb(): return psutil.Process().memory_info().rss / 1024 / 1024
except Exception:
    def rss_mb(): return 0.0

N = int(os.environ.get("GB_ROCKS_N", "100000"))
FANOUT = int(os.environ.get("GB_ROCKS_FANOUT", "8"))
Q = int(os.environ.get("GB_ROCKS_Q", "200"))
LIMIT = int(os.environ.get("GB_ROCKS_LIMIT", "1000"))
DEPTHS = [1, 3, 6]
BENCH = r"C:\Users\freshair\gb_vbench"

def kb(i): return int(i).to_bytes(8, "little")

def main():
    base = rss_mb()
    print(f"P-RocksDB: N={N} fanout={FANOUT} (edges~{N*FANOUT}) q={Q}/depth limit={LIMIT}", flush=True)
    rng = random.Random(42)

    # Build fanout-8 adjacency lists (independent draw, identical stats).
    adj = [None] * N
    for i in range(N):
        adj[i] = np.fromiter((rng.randrange(N) for _ in range(FANOUT)), dtype=np.int64, count=FANOUT)

    dbdir = os.path.join(BENCH, f"rocks_db_{N}")
    shutil.rmtree(dbdir, ignore_errors=True)
    opts = Options(raw_mode=True)            # raw bytes keys/values (no pickle overhead)
    db = Rdict(dbdir, opts)

    t = time.time()
    for i in range(N):
        db[kb(i)] = adj[i].tobytes()          # one KV entry per node = its adjacency list
    db.flush()
    ingest_sec = time.time() - t
    mem = rss_mb() - base
    print(f"ingest: {N} nodes + {N*FANOUT} edges (adjacency KV) {ingest_sec:.1f}s, "
          f"RocksDB RSS delta {mem:.0f} MB", flush=True)

    def traverse(start, d):
        # variable-length a-[*1..d]->b, BFS over KV point lookups, capped at LIMIT.
        # Dedup via visited set to MATCH GenesisBlockDB neighbors() distinct-node
        # semantics (otherwise this would return a walk-with-repeats and hit LIMIT
        # with far less graph exploration — an unfair edge over GenesisBlockDB).
        results = []
        visited = {start}
        frontier = [start]
        for _ in range(d):
            nxt = []
            for node in frontier:
                v = db.get(kb(node))
                if v is None: continue
                for nb in np.frombuffer(v, dtype=np.int64):
                    nb = int(nb)
                    if nb in visited: continue
                    visited.add(nb)
                    results.append(nb)
                    if len(results) >= LIMIT:
                        return results
                    nxt.append(nb)
            frontier = nxt
            if not frontier: break
        return results

    rng2 = random.Random(99)
    per_depth = []
    for d in DEPTHS:
        lats = []
        for _ in range(Q):
            sid = rng2.randrange(N)
            t0 = time.perf_counter()
            traverse(sid, d)
            lats.append((time.perf_counter() - t0) * 1000.0)
        lats = np.array(lats)
        rec = {"depth": d, "p50_us": float(np.percentile(lats, 50) * 1000),
               "p95_us": float(np.percentile(lats, 95) * 1000),
               "p99_us": float(np.percentile(lats, 99) * 1000)}
        per_depth.append(rec)
        print(f"  hop{d}: p50 {rec['p50_us']:.1f}us p95 {rec['p95_us']:.1f}us p99 {rec['p99_us']:.1f}us", flush=True)

    out = {"engine": "RocksDB (embedded KV + adjacency BFS)", "n": N, "fanout": FANOUT,
           "edges": N * FANOUT, "ingest_sec": ingest_sec, "rss_mb": mem, "depths": per_depth}
    json.dump(out, open(os.path.join(BENCH, f"rocksdb_results_{N}.json"), "w"), indent=2)
    db.close()

main()
