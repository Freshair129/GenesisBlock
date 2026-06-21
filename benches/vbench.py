"""
Head-to-head vector benchmark: GenesisBlockDB (hnsw_rs) vs Chroma (hnswlib).

Same corpus, same queries, same k, same metric (L2). Real embeddings from
bge-m3 (1024-dim) via local Ollama -- NOT random vectors, so recall reflects
real semantic clustering.

Modes:
  python vbench.py embed     # collect docs -> embed -> corpus.f32/queries.f32/meta.json/ground_truth.json
  python vbench.py chroma    # run Chroma benchmark -> chroma_results.json
  python vbench.py finalize  # read genesis_results.json + chroma + ground truth -> results.json + table
  python vbench.py all       # embed + chroma
"""
import os, sys, glob, json, time, urllib.request
import numpy as np

BENCH = r"C:\Users\freshair\gb_vbench"
DOCS  = r"G:\GenesisBlock_Dev\GenesisBlock\docs"
MODEL = "bge-m3"
DIM, N, Q, K = 1024, 3000, 200, 10
OLLAMA = "http://localhost:11434/api/embed"

def p(*a): print(*a, flush=True)

def embed_batch(texts):
    body = json.dumps({"model": MODEL, "input": texts}).encode()
    req = urllib.request.Request(OLLAMA, body, {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=600) as r:
        return json.loads(r.read())["embeddings"]

def collect_chunks():
    chunks, seen = [], set()
    files = sorted(glob.glob(os.path.join(DOCS, "**", "*.md"), recursive=True))
    for f in files:
        try:
            txt = open(f, encoding="utf-8", errors="ignore").read()
        except Exception:
            continue
        for line in txt.replace("|", " ").split("\n"):
            s = line.strip().strip("#-*`> ").strip()
            if 40 <= len(s) <= 400 and s not in seen:
                seen.add(s); chunks.append(s)
    return chunks

def do_embed():
    chunks = collect_chunks()
    p(f"collected {len(chunks)} unique chunks from docs")
    need = N + Q
    if len(chunks) < need:
        base, i = chunks[:], 0
        while len(chunks) < need:
            chunks.append(base[i % len(base)] + f" [v{i}]"); i += 1
    chunks = chunks[:need]
    vecs, t = [], time.time()
    B = 64
    for i in range(0, len(chunks), B):
        vecs.extend(embed_batch(chunks[i:i+B]))
        if (i // B) % 5 == 0: p(f"  embedded {min(i+B, len(chunks))}/{len(chunks)}")
    vecs = np.asarray(vecs, dtype=np.float32)
    p(f"embed done in {time.time()-t:.1f}s, shape {vecs.shape}")
    assert vecs.shape == (need, DIM), vecs.shape
    corpus, queries = vecs[:N], vecs[N:N+Q]
    corpus.tofile(os.path.join(BENCH, "corpus.f32"))
    queries.tofile(os.path.join(BENCH, "queries.f32"))
    json.dump({"n": N, "q": Q, "dim": DIM, "k": K, "model": MODEL},
              open(os.path.join(BENCH, "meta.json"), "w"))
    # exact L2 ground truth via ||a-b||^2 = ||a||^2 + ||b||^2 - 2 a.b
    cn = (corpus**2).sum(1); qn = (queries**2).sum(1)
    d2 = qn[:, None] + cn[None, :] - 2.0 * (queries @ corpus.T)
    gt = np.argsort(d2, axis=1)[:, :K]
    json.dump(gt.tolist(), open(os.path.join(BENCH, "ground_truth.json"), "w"))
    p(f"saved corpus({N}) queries({Q}) dim={DIM} + ground_truth (exact L2 top-{K})")

def recall_at_k(topk, gt):
    return float(np.mean([len(set(map(int, topk[i])) & set(map(int, gt[i]))) / len(gt[i])
                          for i in range(len(gt))]))

def do_chroma():
    import chromadb
    meta = json.load(open(os.path.join(BENCH, "meta.json")))
    n, q, dim, k = meta["n"], meta["q"], meta["dim"], meta["k"]
    corpus = np.fromfile(os.path.join(BENCH, "corpus.f32"), dtype=np.float32).reshape(n, dim)
    queries = np.fromfile(os.path.join(BENCH, "queries.f32"), dtype=np.float32).reshape(q, dim)
    client = chromadb.Client()
    try: client.delete_collection("vbench")
    except Exception: pass
    # force L2 to match GenesisBlockDB DistL2
    try:
        col = client.create_collection("vbench", configuration={"hnsw": {"space": "l2"}})
    except Exception:
        col = client.create_collection("vbench", metadata={"hnsw:space": "l2"})
    ids = [str(i) for i in range(n)]
    t = time.time()
    B = 1000
    for i in range(0, n, B):
        col.add(ids=ids[i:i+B], embeddings=corpus[i:i+B].tolist())
    insert_sec = time.time() - t
    lat, topk = [], []
    for qi in range(q):
        t0 = time.perf_counter()
        res = col.query(query_embeddings=[queries[qi].tolist()], n_results=k)
        lat.append((time.perf_counter() - t0) * 1000.0)  # ms
        topk.append([int(x) for x in res["ids"][0]])
    lat = np.asarray(lat)
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    out = {"engine": "Chroma (hnswlib)", "model": MODEL, "n": n, "q": q, "dim": dim, "k": k,
           "insert_sec": insert_sec, "insert_per_sec": n / insert_sec,
           "q_p50_ms": float(np.percentile(lat, 50)), "q_p95_ms": float(np.percentile(lat, 95)),
           "q_mean_ms": float(lat.mean()), "recall_at_k": recall_at_k(topk, gt),
           "durability": "in-memory (ephemeral)"}
    json.dump(out, open(os.path.join(BENCH, "chroma_results.json"), "w"), indent=2)
    p(f"Chroma: insert {out['insert_per_sec']:.0f} vec/s, query p50 {out['q_p50_ms']*1000:.1f}us "
      f"p95 {out['q_p95_ms']*1000:.1f}us, recall@{k} {out['recall_at_k']:.3f}")

def do_qdrant():
    from qdrant_client import QdrantClient, models
    meta = json.load(open(os.path.join(BENCH, "meta.json")))
    n, q, dim, k = meta["n"], meta["q"], meta["dim"], meta["k"]
    corpus = np.fromfile(os.path.join(BENCH, "corpus.f32"), dtype=np.float32).reshape(n, dim)
    queries = np.fromfile(os.path.join(BENCH, "queries.f32"), dtype=np.float32).reshape(q, dim)
    client = QdrantClient(host="localhost", grpc_port=6334, prefer_grpc=True, timeout=300)
    try: client.delete_collection("vbench")
    except Exception: pass
    client.create_collection("vbench",
        vectors_config=models.VectorParams(size=dim, distance=models.Distance.EUCLID))
    t = time.time()
    B = 1000
    for i in range(0, n, B):
        pts = [models.PointStruct(id=j, vector=corpus[j].tolist()) for j in range(i, min(i+B, n))]
        client.upsert("vbench", points=pts, wait=True)
    # wait for async HNSW indexing to finish (bounded)
    for _ in range(240):
        info = client.get_collection("vbench")
        if str(info.status).endswith("green") and (info.indexed_vectors_count or 0) >= n: break
        time.sleep(0.5)
    insert_sec = time.time() - t
    lat, topk = [], []
    for qi in range(q):
        t0 = time.perf_counter()
        try:
            res = client.query_points("vbench", query=queries[qi].tolist(), limit=k).points
        except Exception:
            res = client.search("vbench", query_vector=queries[qi].tolist(), limit=k)
        lat.append((time.perf_counter() - t0) * 1000.0)
        topk.append([int(pt.id) for pt in res])
    lat = np.asarray(lat)
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    out = {"engine": "Qdrant (server, gRPC)", "model": meta.get("model"), "n": n, "q": q, "dim": dim, "k": k,
           "insert_sec": insert_sec, "insert_per_sec": n / insert_sec,
           "q_p50_ms": float(np.percentile(lat, 50)), "q_p95_ms": float(np.percentile(lat, 95)),
           "q_mean_ms": float(lat.mean()), "recall_at_k": recall_at_k(topk, gt),
           "durability": "server, persisted; network/gRPC overhead in latency"}
    json.dump(out, open(os.path.join(BENCH, "qdrant_results.json"), "w"), indent=2)
    p(f"Qdrant: insert {out['insert_per_sec']:.0f} vec/s, query p50 {out['q_p50_ms']*1000:.1f}us "
      f"p95 {out['q_p95_ms']*1000:.1f}us, recall@{k} {out['recall_at_k']:.3f}")

def do_lance():
    # LanceDB — embedded, on-disk (Lance columnar format), Rust core like GenesisBlockDB.
    # Fairest vector comparator: native embedded ANN. Use IVF_HNSW_FLAT (pure HNSW,
    # no quantization) with a single partition + M=16 / ef_construction=200 to match
    # Chroma (hnswlib) and GenesisBlockDB (hnsw_rs) as closely as the API allows.
    import lancedb, pyarrow as pa, shutil
    meta = json.load(open(os.path.join(BENCH, "meta.json")))
    n, q, dim, k = meta["n"], meta["q"], meta["dim"], meta["k"]
    corpus = np.fromfile(os.path.join(BENCH, "corpus.f32"), dtype=np.float32).reshape(n, dim)
    queries = np.fromfile(os.path.join(BENCH, "queries.f32"), dtype=np.float32).reshape(q, dim)
    db_dir = os.path.join(BENCH, "lance_db")
    if os.path.exists(db_dir): shutil.rmtree(db_dir, ignore_errors=True)
    db = lancedb.connect(db_dir)
    schema = pa.schema([pa.field("id", pa.int64()),
                        pa.field("vector", pa.list_(pa.float32(), dim))])
    t = time.time()
    tbl = db.create_table("vbench", schema=schema)
    B = 10000
    for i in range(0, n, B):
        j = min(i + B, n)
        tbl.add(pa.table({"id": pa.array(range(i, j), pa.int64()),
                          "vector": pa.array([corpus[r] for r in range(i, j)],
                                             pa.list_(pa.float32(), dim))}))
    # Build HNSW index (single IVF partition ≈ pure HNSW). Falls back gracefully.
    index_type = "IVF_HNSW_FLAT"
    try:
        tbl.create_index(metric="l2", index_type=index_type, num_partitions=1,
                         m=16, ef_construction=200, replace=True)
    except Exception as e:
        p(f"  (lance HNSW index build failed: {e}; using brute-force scan)")
        index_type = "brute-force (no ANN index)"
    insert_sec = time.time() - t
    EF = 100  # match GenesisBlockDB ef_search=100 for a fair recall–latency point
    def run_query(vec):
        s = tbl.search(vec).distance_type("l2").limit(k)
        try: s = s.nprobes(1)            # single IVF partition ≈ pure HNSW
        except Exception: pass
        try: s = s.ef(EF)                 # query-time HNSW effort == ef_search
        except Exception: pass
        return s.to_list()
    # Warmup: LanceDB is on-disk (Lance columnar) — page-cache the index so we
    # measure steady-state latency, the same warm state Chroma/GenesisBlockDB enjoy
    # by being memory-resident. Without this we'd measure cold disk reads.
    for qi in range(min(20, q)): run_query(queries[qi])
    lat, topk = [], []
    for qi in range(q):
        t0 = time.perf_counter()
        res = run_query(queries[qi])
        lat.append((time.perf_counter() - t0) * 1000.0)  # ms
        topk.append([int(row["id"]) for row in res])
    lat = np.asarray(lat)
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    out = {"engine": "LanceDB (IVF_HNSW_FLAT, embedded)", "model": meta.get("model"),
           "n": n, "q": q, "dim": dim, "k": k, "index_type": index_type, "ef_search": EF,
           "insert_sec": insert_sec, "insert_per_sec": n / insert_sec,
           "q_p50_ms": float(np.percentile(lat, 50)), "q_p95_ms": float(np.percentile(lat, 95)),
           "q_mean_ms": float(lat.mean()), "recall_at_k": recall_at_k(topk, gt),
           "durability": "on-disk (Lance columnar, persisted)"}
    json.dump(out, open(os.path.join(BENCH, "lance_results.json"), "w"), indent=2)
    p(f"LanceDB: insert {out['insert_per_sec']:.0f} vec/s, query p50 {out['q_p50_ms']*1000:.1f}us "
      f"p95 {out['q_p95_ms']*1000:.1f}us, recall@{k} {out['recall_at_k']:.3f} [{index_type}]")

def _p50_us(r):  return r["q_p50_us"] if "q_p50_us" in r else r["q_p50_ms"] * 1000
def _p95_us(r):  return r["q_p95_us"] if "q_p95_us" in r else r["q_p95_ms"] * 1000

def do_finalize():
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    meta = json.load(open(os.path.join(BENCH, "meta.json")))
    engines = []
    g = json.load(open(os.path.join(BENCH, "genesis_results.json")))
    g["recall_at_k"] = recall_at_k(g["topk"], gt); g.pop("topk", None)
    engines.append(g)
    for fn in ("chroma_results.json", "qdrant_results.json", "lance_results.json"):
        path = os.path.join(BENCH, fn)
        if os.path.exists(path): engines.append(json.load(open(path)))
    json.dump({e["engine"]: e for e in engines}, open(os.path.join(BENCH, "results.json"), "w"), indent=2)
    k = g["k"]
    p(f"\n========= RESULT ({meta.get('model')}, n={meta['n']}, L2, same vectors) =========")
    hdr = f"{'metric':<16}" + "".join(f"{e['engine']:>26}" for e in engines)
    p(hdr); p("-" * len(hdr))
    p(f"{'insert (vec/s)':<16}" + "".join(f"{e['insert_per_sec']:>26.0f}" for e in engines))
    p(f"{'query p50 (us)':<16}" + "".join(f"{_p50_us(e):>26.1f}" for e in engines))
    p(f"{'query p95 (us)':<16}" + "".join(f"{_p95_us(e):>26.1f}" for e in engines))
    p(f"{'recall@'+str(k):<16}" + "".join(f"{e['recall_at_k']:>26.3f}" for e in engines))
    p("\nGenesisBlockDB insert = durable batched WAL fsync; Chroma = in-memory ephemeral;")
    p("Qdrant = server (persisted) with network/gRPC overhead in query latency.")

def do_synth(n):
    # Synthetic clustered vectors for scale tests where we lack N diverse real
    # texts. Gaussian blobs around random centroids, unit-normalized (like bge-m3).
    # Standard ANN-quality workload; identical vectors are fed to both engines.
    rng = np.random.default_rng(42)
    n_clusters = max(16, n // 500)
    centers = rng.standard_normal((n_clusters, DIM)).astype(np.float32)
    assign = rng.integers(0, n_clusters, size=n + Q)
    pts = centers[assign] + (0.18 * rng.standard_normal((n + Q, DIM))).astype(np.float32)
    pts /= (np.linalg.norm(pts, axis=1, keepdims=True) + 1e-9)
    pts = pts.astype(np.float32)
    corpus, queries = pts[:n], pts[n:n+Q]
    corpus.tofile(os.path.join(BENCH, "corpus.f32"))
    queries.tofile(os.path.join(BENCH, "queries.f32"))
    json.dump({"n": n, "q": Q, "dim": DIM, "k": K, "model": f"synthetic-clustered ({n_clusters} clusters, dim {DIM})"},
              open(os.path.join(BENCH, "meta.json"), "w"))
    cn = (corpus**2).sum(1); qn = (queries**2).sum(1)
    d2 = qn[:, None] + cn[None, :] - 2.0 * (queries @ corpus.T)
    gt = np.argsort(d2, axis=1)[:, :K]
    json.dump(gt.tolist(), open(os.path.join(BENCH, "ground_truth.json"), "w"))
    p(f"synth: {n} corpus + {Q} queries, dim {DIM}, {n_clusters} clusters + exact L2 ground truth")

def do_frontier():
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    fr = json.load(open(os.path.join(BENCH, "genesis_frontier.json")))
    pts = [{"ef_search": p["ef_search"], "p50_us": p["q_p50_us"], "p95_us": p["q_p95_us"],
            "recall": recall_at_k(p["topk"], gt)} for p in fr["points"]]
    out = {"genesis": {"engine": "GenesisBlockDB (hnsw_rs)", "ef_construction": fr["ef_construction"],
                       "n": fr["n"], "dim": fr["dim"], "points": pts}}
    for fn, key in (("chroma_results.json", "chroma"), ("qdrant_results.json", "qdrant"),
                    ("lance_results.json", "lance")):
        path = os.path.join(BENCH, fn)
        if os.path.exists(path):
            r = json.load(open(path)); out[key] = {"p50_us": _p50_us(r), "recall": r["recall_at_k"]}
    json.dump(out, open(os.path.join(BENCH, "frontier_results.json"), "w"), indent=2)
    p(f"\n===== RECALL-LATENCY FRONTIER (GenesisBlockDB, n={fr['n']}, ef_construction={fr['ef_construction']}) =====")
    p(f"{'ef_search':>10}{'p50 (us)':>12}{'p95 (us)':>12}{'recall@'+str(fr['k']):>12}")
    for pt in pts:
        p(f"{pt['ef_search']:>10}{pt['p50_us']:>12.1f}{pt['p95_us']:>12.1f}{pt['recall']:>12.3f}")
    for key in ("chroma", "qdrant", "lance"):
        if key in out: p(f"  ref {key:<6} p50 {out[key]['p50_us']:.1f}us  recall {out[key]['recall']:.3f}")

def do_scalerow():
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    g = json.load(open(os.path.join(BENCH, "genesis_results.json")))
    r = recall_at_k(g["topk"], gt)
    row = {"engine": "GenesisBlockDB", "n": g["n"], "build_sec": g.get("build_sec"), "rss_mb": g.get("peak_rss_mb"),
           "insert_per_sec": g["insert_per_sec"], "q_p50_us": g["q_p50_us"], "recall": r}
    cr = None
    cpath = os.path.join(BENCH, "chroma_results.json")
    if os.path.exists(cpath): cr = json.load(open(cpath))
    p(f"GenesisBlockDB  N={g['n']:>9,}  build={g.get('build_sec',0):7.1f}s  RSS={g.get('peak_rss_mb','?'):>6} MB  "
      f"insert={g['insert_per_sec']:7.0f}/s  p50={g['q_p50_us']:8.1f}us  recall={r:.3f}")
    if cr:
        p(f"Chroma     N={cr['n']:>9,}  build={cr['insert_sec']:7.1f}s  RSS={'    -':>6}     "
          f"insert={cr['insert_per_sec']:7.0f}/s  p50={cr['q_p50_ms']*1000:8.1f}us  recall={cr['recall_at_k']:.3f}")
    # append to a running scale log
    log = os.path.join(BENCH, "scale_log.jsonl")
    with open(log, "a") as f: f.write(json.dumps(row) + "\n")

mode = sys.argv[1] if len(sys.argv) > 1 else "all"
if mode == "synth": do_synth(int(sys.argv[2]))
if mode == "frontier": do_frontier()
if mode == "scalerow": do_scalerow()
if mode in ("embed", "all"): do_embed()
if mode in ("chroma", "all"): do_chroma()
if mode == "qdrant": do_qdrant()
if mode == "lance": do_lance()
if mode == "finalize": do_finalize()
