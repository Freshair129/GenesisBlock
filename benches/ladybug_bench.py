"""
MARK XVI — LadybugDB head-to-head (embedded <-> embedded graph engine).
LadybugDB is the community fork of Kuzu (continues Kuzu after its main dev
stopped): property-graph, Cypher, columnar + CSR adjacency, HNSW + FTS, ACID.
It sits at exactly GenesisBlockDB's niche (embedded graph+vector, agentic memory
/ hybrid RAG), so it is the single most on-target competitor. API is
kuzu-compatible, so this mirrors benches/kuzu_bench.py with a swapped import.
Same topology params as P22/P26 (N nodes, fanout-8 random), depths {1,3,6}.
"""
import os, json, time, random, shutil
import numpy as np
import real_ladybug as lbug
try:
    import psutil
    def rss_mb(): return psutil.Process().memory_info().rss / 1024 / 1024
except Exception:
    def rss_mb(): return 0.0

N = int(os.environ.get("GB_LADYBUG_N", "100000"))
FANOUT = int(os.environ.get("GB_LADYBUG_FANOUT", "8"))
Q = int(os.environ.get("GB_LADYBUG_Q", "200"))
LIMIT = int(os.environ.get("GB_LADYBUG_LIMIT", "1000"))
DEPTHS = [1, 3, 6]
BENCH = r"C:\Users\freshair\gb_vbench"

def main():
    base = rss_mb()
    print(f"P-Ladybug: N={N} fanout={FANOUT} (edges~{N*FANOUT}) q={Q}/depth limit={LIMIT}", flush=True)
    rng = random.Random(42)

    # write CSVs (idiomatic bulk path = COPY FROM), identical to kuzu_bench
    ncsv = os.path.join(BENCH, "lb_nodes.csv")
    ecsv = os.path.join(BENCH, "lb_edges.csv")
    with open(ncsv, "w") as f:
        for i in range(N): f.write(f"{i}\n")
    with open(ecsv, "w") as f:
        for i in range(N):
            for _ in range(FANOUT):
                f.write(f"{i},{rng.randrange(N)}\n")

    dbdir = os.path.join(BENCH, f"ladybug_db_{N}")
    shutil.rmtree(dbdir, ignore_errors=True)
    db = lbug.Database(dbdir)
    conn = lbug.Connection(db)
    conn.execute("CREATE NODE TABLE V(gid INT64, PRIMARY KEY (gid))")
    conn.execute("CREATE REL TABLE LINK(FROM V TO V)")

    t = time.time()
    conn.execute(f'COPY V FROM "{ncsv.replace(chr(92), "/")}"')
    conn.execute(f'COPY LINK FROM "{ecsv.replace(chr(92), "/")}"')
    ingest_sec = time.time() - t
    mem = rss_mb() - base
    print(f"ingest: {N} nodes + {N*FANOUT} edges COPY {ingest_sec:.1f}s, Ladybug RSS delta {mem:.0f} MB", flush=True)

    rng2 = random.Random(99)
    per_depth = []
    for d in DEPTHS:
        cy = f"MATCH (a:V {{gid:$id}})-[:LINK*1..{d}]->(b:V) RETURN b.gid LIMIT {LIMIT}"
        stmt = conn.prepare(cy)  # prepare once -> fair vs a compiled traversal call
        lats = []
        for _ in range(Q):
            sid = rng2.randrange(N)
            t0 = time.perf_counter()
            res = conn.execute(stmt, {"id": sid})
            try:
                while res.has_next(): res.get_next()
            except Exception:
                pass
            lats.append((time.perf_counter() - t0) * 1000.0)
        lats = np.array(lats)
        rec = {"depth": d, "p50_us": float(np.percentile(lats, 50) * 1000),
               "p95_us": float(np.percentile(lats, 95) * 1000),
               "p99_us": float(np.percentile(lats, 99) * 1000)}
        per_depth.append(rec)
        print(f"  hop{d}: p50 {rec['p50_us']:.1f}us p95 {rec['p95_us']:.1f}us p99 {rec['p99_us']:.1f}us", flush=True)

    out = {"engine": "LadybugDB (embedded, Kuzu fork)", "n": N, "fanout": FANOUT,
           "edges": N * FANOUT, "ingest_sec": ingest_sec, "rss_mb": mem, "depths": per_depth}
    json.dump(out, open(os.path.join(BENCH, f"ladybug_results_{N}.json"), "w"), indent=2)

main()
