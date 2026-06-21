"""
P23 — Neo4j head-to-head graph traversal (same topology params as P22).
Embedded GenesisBlockDB vs client-server Neo4j (bolt + JVM). Caveat: Neo4j latency
includes the bolt round-trip + JVM; memory is JVM heap+pagecache, not RSS.
"""
import os, sys, json, time, random, subprocess
import numpy as np
from neo4j import GraphDatabase

N = int(os.environ.get("GB_NEO_N", "100000"))
FANOUT = int(os.environ.get("GB_NEO_FANOUT", "8"))
Q = int(os.environ.get("GB_NEO_Q", "200"))
LIMIT = int(os.environ.get("GB_NEO_LIMIT", "1000"))
DEPTHS = [1, 3, 6]
BENCH = r"C:\Users\freshair\gb_vbench"
drv = GraphDatabase.driver("bolt://localhost:7687")

def run(cy, **kw):
    with drv.session() as s:
        return s.run(cy, **kw).consume()

def query_ids(cy, **kw):
    with drv.session() as s:
        return [r[0] for r in s.run(cy, **kw)]

def docker_mem_mb(name):
    try:
        out = subprocess.run(["docker", "stats", "--no-stream", "--format", "{{.MemUsage}}", name],
                             capture_output=True, text=True, timeout=20).stdout.strip()
        # e.g. "1.23GiB / 31GiB"
        used = out.split("/")[0].strip()
        v = float(''.join(c for c in used if (c.isdigit() or c == '.')))
        if "GiB" in used: return v * 1024
        if "MiB" in used: return v
        return v
    except Exception:
        return None

def main():
    print(f"P23 Neo4j: N={N} fanout={FANOUT} (edges~{N*FANOUT}) q={Q}/depth limit={LIMIT}", flush=True)
    try:
        run("MATCH (n) CALL (n) { DETACH DELETE n } IN TRANSACTIONS OF 10000 ROWS")
    except Exception:
        try: run("MATCH (n) CALL { WITH n DETACH DELETE n } IN TRANSACTIONS OF 10000 ROWS")
        except Exception: run("MATCH (n) DETACH DELETE n")
    run("CREATE INDEX v_gid IF NOT EXISTS FOR (n:V) ON (n.gid)")
    rng = random.Random(42)

    # ingest nodes
    t = time.time()
    B = 10000
    for i in range(0, N, B):
        run("UNWIND range($a,$b) AS x CREATE (:V {gid:x})", a=i, b=min(i+B, N) - 1)
    node_sec = time.time() - t

    # ingest edges (batched MATCH+CREATE; index-backed)
    t = time.time()
    B2 = 5000
    pairs, total = [], 0
    def flush(p):
        run("UNWIND $p AS e MATCH (a:V {gid:e[0]}),(b:V {gid:e[1]}) CREATE (a)-[:LINK]->(b)", p=p)
    for i in range(N):
        for _ in range(FANOUT):
            pairs.append([i, rng.randrange(N)])
            if len(pairs) >= B2:
                flush(pairs); total += len(pairs); pairs = []
    if pairs:
        flush(pairs); total += len(pairs)
    edge_sec = time.time() - t
    mem = docker_mem_mb("gb-neo4j")
    print(f"ingest: {N} nodes {node_sec:.1f}s, {total} edges {edge_sec:.1f}s, Neo4j mem {mem} MB", flush=True)

    # traversal
    rng2 = random.Random(99)
    per_depth = []
    for d in DEPTHS:
        cy = f"MATCH (n:V {{gid:$id}})-[:LINK*1..{d}]->(m) RETURN m.gid AS g LIMIT {LIMIT}"
        lats = []
        for _ in range(Q):
            sid = rng2.randrange(N)
            t0 = time.perf_counter()
            _ = query_ids(cy, id=sid)
            lats.append((time.perf_counter() - t0) * 1000.0)
        lats = np.array(lats)
        rec = {"depth": d, "p50_us": float(np.percentile(lats, 50) * 1000),
               "p95_us": float(np.percentile(lats, 95) * 1000),
               "p99_us": float(np.percentile(lats, 99) * 1000)}
        per_depth.append(rec)
        print(f"  hop{d}: p50 {rec['p50_us']:.1f}us p95 {rec['p95_us']:.1f}us p99 {rec['p99_us']:.1f}us", flush=True)

    out = {"engine": "Neo4j (server, bolt)", "n": N, "fanout": FANOUT, "edges": total,
           "node_ingest_sec": node_sec, "edge_ingest_sec": edge_sec, "mem_mb": mem, "depths": per_depth}
    json.dump(out, open(os.path.join(BENCH, f"neo4j_results_{N}.json"), "w"), indent=2)

main()
