"""
MARK XIV — DuckDB + graph head-to-head (embedded <-> embedded).
DuckDB is an embedded columnar analytical engine; "graph" traversal is expressed
as a recursive CTE over an edges table (the idiomatic DuckDB-as-graph approach).
Same topology params as P22/P26 (N nodes, fanout-8 random), depths {1,3,6}.
Mirrors benches/kuzu_bench.py so the GenesisBlockDB side (graph_results_{N}.json from
graph-bench) is directly comparable.
"""
import os, json, time, random
import numpy as np
import duckdb
try:
    import psutil
    def rss_mb(): return psutil.Process().memory_info().rss / 1024 / 1024
except Exception:
    def rss_mb(): return 0.0

N = int(os.environ.get("GB_DUCK_N", "100000"))
FANOUT = int(os.environ.get("GB_DUCK_FANOUT", "8"))
Q = int(os.environ.get("GB_DUCK_Q", "200"))
LIMIT = int(os.environ.get("GB_DUCK_LIMIT", "1000"))
DEPTHS = [1, 3, 6]
BENCH = r"C:\Users\freshair\gb_vbench"

def main():
    base = rss_mb()
    print(f"P-DuckDB: N={N} fanout={FANOUT} (edges~{N*FANOUT}) q={Q}/depth limit={LIMIT}", flush=True)
    rng = random.Random(42)

    # Build the same fanout-8 random edge list (independent draw, identical stats).
    froms = np.empty(N * FANOUT, dtype=np.int64)
    tos = np.empty(N * FANOUT, dtype=np.int64)
    idx = 0
    for i in range(N):
        for _ in range(FANOUT):
            froms[idx] = i; tos[idx] = rng.randrange(N); idx += 1

    con = duckdb.connect()  # in-memory embedded
    con.execute("CREATE TABLE edges(from_id BIGINT, to_id BIGINT)")
    t = time.time()
    # Bulk insert via Arrow/numpy registration (DuckDB's fast vectorized path).
    rel = con.from_arrow(__import__("pyarrow").table({"from_id": froms, "to_id": tos}))
    con.execute("INSERT INTO edges SELECT * FROM rel")
    con.execute("CREATE INDEX idx_from ON edges(from_id)")  # ART index for point lookups
    ingest_sec = time.time() - t
    mem = rss_mb() - base
    print(f"ingest: {N} nodes + {N*FANOUT} edges {ingest_sec:.1f}s (+index), DuckDB RSS delta {mem:.0f} MB", flush=True)

    rng2 = random.Random(99)
    per_depth = []
    for d in DEPTHS:
        # Recursive CTE = variable-length traversal a-[*1..d]->b, capped at LIMIT.
        sql = (
            "WITH RECURSIVE reach(node, depth) AS ("
            "  SELECT to_id AS node, 1 AS depth FROM edges WHERE from_id = ? "
            "  UNION ALL "
            "  SELECT e.to_id, r.depth + 1 FROM reach r JOIN edges e ON e.from_id = r.node "
            "  WHERE r.depth < ? "
            ") SELECT node FROM reach LIMIT ?"
        )
        lats = []
        for _ in range(Q):
            sid = rng2.randrange(N)
            t0 = time.perf_counter()
            con.execute(sql, [sid, d, LIMIT]).fetchall()
            lats.append((time.perf_counter() - t0) * 1000.0)
        lats = np.array(lats)
        rec = {"depth": d, "p50_us": float(np.percentile(lats, 50) * 1000),
               "p95_us": float(np.percentile(lats, 95) * 1000),
               "p99_us": float(np.percentile(lats, 99) * 1000)}
        per_depth.append(rec)
        print(f"  hop{d}: p50 {rec['p50_us']:.1f}us p95 {rec['p95_us']:.1f}us p99 {rec['p99_us']:.1f}us", flush=True)

    out = {"engine": "DuckDB (embedded, recursive CTE)", "n": N, "fanout": FANOUT,
           "edges": N * FANOUT, "ingest_sec": ingest_sec, "rss_mb": mem, "depths": per_depth}
    json.dump(out, open(os.path.join(BENCH, f"duckdb_results_{N}.json"), "w"), indent=2)

main()
