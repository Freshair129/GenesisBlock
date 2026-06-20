"""
Head-to-head vector benchmark: GenesisDB (hnsw_rs) vs Chroma (hnswlib).

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
    # force L2 to match GenesisDB DistL2
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

def do_finalize():
    gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
    g = json.load(open(os.path.join(BENCH, "genesis_results.json")))
    c = json.load(open(os.path.join(BENCH, "chroma_results.json")))
    g["recall_at_k"] = recall_at_k(g["topk"], gt)
    g.pop("topk", None)
    combined = {"genesis": g, "chroma": c}
    json.dump(combined, open(os.path.join(BENCH, "results.json"), "w"), indent=2)
    k = g["k"]
    p("\n================ RESULT (bge-m3 1024-dim, L2, same vectors) ================")
    p(f"{'metric':<26}{'GenesisDB (hnsw_rs)':>22}{'Chroma (hnswlib)':>22}")
    p("-"*70)
    p(f"{'insert (vec/s)':<26}{g['insert_per_sec']:>22.0f}{c['insert_per_sec']:>22.0f}")
    p(f"{'query p50':<26}{g['q_p50_us']:>19.1f}us{c['q_p50_ms']*1000:>19.1f}us")
    p(f"{'query p95':<26}{g['q_p95_us']:>19.1f}us{c['q_p95_ms']*1000:>19.1f}us")
    p(f"{'recall@'+str(k):<26}{g['recall_at_k']:>22.3f}{c['recall_at_k']:>22.3f}")
    p(f"\nNote: GenesisDB insert = durable per-op WAL fsync; Chroma = in-memory batch add.")
    p("Query latency & recall are the apples-to-apples HNSW-vs-HNSW metrics.")

mode = sys.argv[1] if len(sys.argv) > 1 else "all"
if mode in ("embed", "all"): do_embed()
if mode in ("chroma", "all"): do_chroma()
if mode == "finalize": do_finalize()
