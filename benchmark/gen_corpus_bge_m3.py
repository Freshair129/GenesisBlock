#!/usr/bin/env python3
"""Build a REAL-embedding corpus for the moat bench (WP-3.3 follow-up 2).

The moat verdict (docs/REPORT--G3-MOAT-VERDICT.md) was measured on synthetic
seeded unit vectors and carried an explicit caveat: random unit vectors are
isotropic, while real embeddings are anisotropic and clustered, which changes
HNSW graph quality (and therefore the engine's side of the ratio) but NOT the
baseline's full-scan cost. This script produces the real half of that A/B.

Corpus provenance (deliberately local + reproducible, no downloads):
prose extracted from THIS repository — markdown documents and the natural
language inside `///`, `//!`, `//`, `#` comments across src/, tests/,
benches/, mcp/, benchmark/, sdk/, dashboard/src/. Extraction is
deterministic (sorted file walk, fixed chunk rules), so re-running on the
same commit reproduces the same corpus.

    python benchmark/gen_corpus_bge_m3.py --out benchmark/fixtures/corpus_bge_m3

Writes <out>.f32 (little-endian f32, count*dim, L2-normalized) and
<out>.manifest.json (model, dim, count, sha256, extraction rules, commit).
The bench consumes them via GB_MOAT_VECTORS=<out>.f32.
"""

import argparse
import hashlib
import json
import pathlib
import re
import struct
import subprocess
import sys
import urllib.error
import urllib.request

ROOTS = ["docs", "src", "tests", "benches", "mcp", "benchmark", "sdk", "dashboard/src"]
EXTS = {".md", ".rs", ".py", ".mjs", ".ts", ".tsx"}
SKIP = ("node_modules", "target", ".git", "__pycache__")
MIN_WORDS, MAX_WORDS = 8, 120
COMMENT_RE = re.compile(r"^\s*(///|//!|//|#)\s?")
SPLIT_RE = re.compile(r"(?<=[.!?])\s+|\n\s*\n")


def extract_chunks(repo: pathlib.Path) -> list[str]:
    """Deterministic prose extraction. Sorted walk => stable ordering."""
    chunks: list[str] = []
    for root in ROOTS:
        base = repo / root
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            if path.suffix not in EXTS or any(s in str(path) for s in SKIP):
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            if path.suffix == ".md":
                body = text
            else:
                body = "\n".join(
                    COMMENT_RE.sub("", line)
                    for line in text.splitlines()
                    if COMMENT_RE.match(line)
                )
            for sentence in SPLIT_RE.split(body):
                words = sentence.split()
                if MIN_WORDS <= len(words) <= MAX_WORDS:
                    chunks.append(" ".join(words))
    # Dedupe, preserving first-seen order (boilerplate repeats across files).
    return list(dict.fromkeys(chunks))


def embed_batch(host: str, model: str, batch: list[str]) -> list[list[float]]:
    payload = json.dumps({"model": model, "input": batch}).encode()
    req = urllib.request.Request(
        f"{host}/api/embed", data=payload, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=600) as resp:
        return json.loads(resp.read())["embeddings"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="benchmark/fixtures/corpus_bge_m3")
    ap.add_argument("--model", default="bge-m3")
    ap.add_argument("--host", default="http://localhost:11434")
    ap.add_argument("--batch", type=int, default=32)
    ap.add_argument("--limit", type=int, default=0, help="0 = all chunks")
    args = ap.parse_args()

    repo = pathlib.Path(__file__).resolve().parent.parent
    chunks = extract_chunks(repo)
    if args.limit:
        chunks = chunks[: args.limit]
    if not chunks:
        print("no chunks extracted", file=sys.stderr)
        return 1
    print(f"extracted {len(chunks)} unique chunks from {len(ROOTS)} roots")

    # Probe the dim with one call so the on-disk row size is known up front —
    # that is what makes the append-as-you-go / resume accounting exact.
    dim = len(embed_batch(args.host, args.model, chunks[:1])[0])
    row_bytes = dim * 4
    out_path = pathlib.Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    bin_path = out_path.with_suffix(".f32")

    # Embeddings are appended batch-by-batch rather than buffered to the end:
    # a 10k-chunk run is tens of minutes, and an interrupted run must not throw
    # all of it away. A partial file is a valid prefix of the final one, so a
    # re-run resumes at exactly the chunk after the last whole row written.
    done = 0
    if bin_path.exists():
        size = bin_path.stat().st_size
        done = size // row_bytes
        if size % row_bytes:  # torn write: drop the partial row
            with bin_path.open("r+b") as fh:
                fh.truncate(done * row_bytes)
        done = min(done, len(chunks))
        if done:
            print(f"resuming: {done}/{len(chunks)} already embedded")

    mode = "ab" if done else "wb"
    with bin_path.open(mode) as fh:
        for start in range(done, len(chunks), args.batch):
            batch = chunks[start : start + args.batch]
            try:
                vecs = embed_batch(args.host, args.model, batch)
            except (urllib.error.URLError, TimeoutError) as exc:
                print(f"embed failed at chunk {start}: {exc}", file=sys.stderr)
                print(f"partial file kept ({start} rows) — re-run to resume", file=sys.stderr)
                return 1
            if any(len(v) != dim for v in vecs):
                print("ragged embedding dims from the model", file=sys.stderr)
                return 1
            # L2-normalize: the engine normalizes for Cosine collections and the
            # baseline ranks by dot product, so both sides must see unit vectors
            # — otherwise the comparison measures normalization, not the store.
            for vec in vecs:
                norm = sum(x * x for x in vec) ** 0.5 or 1.0
                fh.write(struct.pack(f"<{dim}f", *[x / norm for x in vec]))
            fh.flush()
            if (start // args.batch) % 20 == 0:
                print(f"  embedded {min(start + args.batch, len(chunks))}/{len(chunks)}", flush=True)

    blob = bin_path.read_bytes()
    count = len(blob) // row_bytes

    try:
        commit = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        commit = "unknown"

    manifest = {
        "model": args.model,
        "model_host": "ollama",
        "dim": dim,
        "count": count,
        "normalized": "l2",
        "sha256": hashlib.sha256(blob).hexdigest(),
        "bytes": len(blob),
        "source_commit": commit,
        "corpus": "prose extracted from this repository (markdown + source comments)",
        "extraction": {
            "roots": ROOTS,
            "extensions": sorted(EXTS),
            "min_words": MIN_WORDS,
            "max_words": MAX_WORDS,
            "dedupe": "first-seen order preserved",
        },
    }
    manifest_path = out_path.with_suffix(".manifest.json")
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"wrote {bin_path} ({len(blob)} bytes, {count}x{dim})")
    print(f"wrote {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
