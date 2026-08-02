---
proposed_id: PLAN--P34-HELIXDB-BENCH-SCOPE
type: plan
status: current
aliases:
  - PLAN
  - P34
tier: process
cluster: implementation_flow
role: "Scoping: head-to-head benchmark vs HelixDB v3.0.7 (Rust, Apache-2.0, HelixQL)"
phase: 34
date: 2026-07-03
proposed_by: agent
related:
  - AUDIT--P23-NEO4J-HEAD-TO-HEAD
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# PLAN — P34 HelixDB Head-to-Head Bench Scope

## 0. Verdict up front

**Not embeddable. Server-only, Docker-mandatory.** `helix start dev` in the
HelixDB v3 CLI unconditionally shells out to `docker run` / `podman run` — there
is no native-process code path. The public `HelixDB/helix-db` repo does not
contain the storage-engine source at all (only `helix-cli`, `metrics`,
`sdks/rust`); the engine (`HelixGraphEngine`, LMDB or LSM+object-store,
`HelixGateway`) ships only as the closed container image
`ghcr.io/helixdb/enterprise-dev`. This makes the comparison structurally the
**same shape as P23 (Neo4j)**, not P26/P30 (Kuzu/LadybugDB) — embedded
GenesisBlockDB vs. a server/container competitor, with an unavoidable
protocol/HTTP tax baked into every HelixDB number. Runnable on the Windows 10
bench host **only via Docker Desktop + WSL2**, which the host's build
(10.0.19045) satisfies, so it is runnable, but the honest framing must say so
loudly (§3, §0 above).

All claims below carry a source URL and access date (2026-07-03, fetched
2026-07-03T06:15 UTC unless noted). Anything not directly verified is marked
**unknown**.

---

## 1. Deployment model

- **Repo / license / version confirmed live:** `HelixDB/helix-db`, Apache-2.0
  (verified by fetching the raw `LICENSE` file directly — a third-party listing
  at dbdb.io claims GPLv3, which is stale/wrong; the repo's own `LICENSE` file
  is authoritative), 5,548 stars, default branch `main`, latest tag `v3.0.7`
  published 2026-07-02T13:52:28Z (one day before this scoping). Source:
  [github.com/HelixDB/helix-db](https://github.com/HelixDB/helix-db) (GitHub API,
  fetched 2026-07-03).
- **v3.0.7 release body is literally "cli fixes (#943)"** — this is a CLI
  patch release, not an engine release. Source:
  [releases/tag/v3.0.7](https://github.com/HelixDB/helix-db/releases/tag/v3.0.7).
- **Repo workspace = CLI + SDK only.** Root `Cargo.toml`:
  `members = ["helix-cli", "metrics", "sdks/rust"]`. There is no
  `helix-server`/`helix-engine`/storage crate in this repo. Source: GitHub API
  contents fetch of `Cargo.toml`, 2026-07-03.
- **`helix start dev` is a Docker/Podman wrapper, not a process launcher.**
  `helix-cli/src/local_runtime.rs` builds `Command::new(self.runtime.binary())`
  where `runtime.binary()` is `"docker"` or `"podman"`, with args
  `["run", "-d", "--restart", "unless-stopped", "--name", <name>, "-p",
  "<port>:8080", <image>]`. The image is `ghcr.io/helixdb/enterprise-dev`.
  Source: raw fetch of `helix-cli/src/local_runtime.rs` main branch, 2026-07-03.
- **The engine's actual code is not public.** The `enterprise-dev` container
  runs `HelixGraphEngine` (LMDB storage backend, per third-party
  descriptions) behind `HelixGateway` on port 6969→8080; none of that engine
  source is in `HelixDB/helix-db`. Historical note: a third-party fork
  description states "the original open-source v1 version used LMDB… limited
  to sequential writes"; the current commercial/cloud path is described
  elsewhere as LSM-based storage on object storage with a single writer +
  auto-scaling readers. Sources:
  [DeepWiki HelixDB/helix-db](https://deepwiki.com/HelixDB/helix-db) (fetched
  2026-07-03, describes repo as "CLI tooling and multi-language SDK layer…
  the core database engine runs within managed container images"); web search
  synthesis citing HelixDB Cloud architecture docs (fetched 2026-07-03) —
  **the LSM/object-storage claim for the exact `enterprise-dev` local image is
  not independently confirmed from primary source and should be marked
  unknown for certainty; what IS confirmed is that the engine binary/source is
  not in the public repo either way.**
- **Windows release asset exists but is the CLI, not the engine.**
  `v3.0.7` ships `helix-x86_64-pc-windows-msvc.exe` alongside macOS/Linux
  builds — this is the `helix-cli` binary (matches the "cli fixes" release
  note and the workspace membership above), not a standalone DB server binary.
  Source: GitHub API `releases/tags/v3.0.7` asset list, 2026-07-03.
- **Minimal way to run on Windows 10:** Docker Desktop with the WSL2 backend
  is the only supported local path — there is no native Windows/Linux/macOS
  process mode, no WSL2-free container runtime documented, and no embedded
  library mode in any SDK (see §2c). Docker Desktop on Windows 10 requires
  Pro/Enterprise/Education, build ≥19045 (21H2), SLAT + BIOS virtualization.
  Our bench host is **Windows 10 Pro, build 19045** — right at the minimum
  supported build, so it qualifies, but leaves no headroom (any Windows Update
  regression risk is real). Sources:
  [docs.helix-db.com/database/local-development](https://docs.helix-db.com/database/local-development)
  (fetched 2026-07-03); Docker install docs (WebSearch synthesis, fetched
  2026-07-03, docs.docker.com/desktop/setup/install/windows-install/).
- **No WSL2-free Windows path found; not exhaustively ruled out.** I did not
  find a documented "native Windows daemon" mode anywhere (docs, README, CLI
  source). Marking this **unknown-but-improbable** rather than a hard "no",
  since the docs site is not exhaustive and the vendor could have an
  undocumented flag — but the CLI source (`local_runtime.rs`) shows no
  alternate code path, which is stronger evidence than doc silence.

**Answer to Q1:** Server/container-only. No in-process embedding exists at any
supported layer (CLI, Rust/TS/Python/Go SDKs are all HTTP clients — see §2c).
Windows 10 route = Docker Desktop + WSL2, which the bench host's build number
just barely satisfies.

---

## 2. Data model + query surface

### (a) Bulk load 100k nodes / 800k edges

- No COPY-equivalent found in the public repo or docs for the Rust/TS/Go SDKs.
- The **Python** ecosystem package `helix-py` (`HelixDB/helix-py`, separate
  repo) ships a `loader.py` that accepts `.parquet`, `.fvecs`, and `.csv` files
  plus a column list, and "the loader does the rest." No batching size,
  throughput numbers, or COPY-style bulk-format contract is documented — it
  reads as a convenience wrapper that still issues individual/batched
  `POST /v1/query` insert calls, not a server-side bulk-file-ingest command
  analogous to Kuzu/LadybugDB's `COPY FROM`. **This is a real ingest-comparability
  risk** (see §3): if `helix-py`'s loader is just chunked HTTP inserts, our
  800k-edge ingest number will be dominated by HTTP+serialization overhead in
  a way LadybugDB's native `COPY FROM` never pays, and that needs to be
  reported as a caveat, not hidden. Source:
  [github.com/HelixDB/helix-py](https://github.com/HelixDB/helix-py) README
  (fetched via WebFetch, 2026-07-03) — **loader internals (batch size, protocol)
  not verified; marked unknown pending source read of `helix/loader.py`.**

### (b) Variable-depth traversal in HelixQL

HelixQL is a builder/DSL (not a text query language parsed at runtime like
Cypher/HQL) — queries are expressed as chained builder calls in each SDK
language and compiled/sent as a JSON AST to `POST /v1/query`. Confirmed
building blocks (from `docs.helix-db.com/database/querying-guide/traversals`
and `.../advanced`, fetched 2026-07-03):

```
g().n(NodeRef.var("a"))
   .repeat(RepeatConfig.new(sub().out("LINK")).times(6).maxDepth(6))
   .valueMap(["$id"])
   .limit(1000)
```

- `.repeat(sub, ...).times(n)` walks a sub-traversal exactly `n` times;
  `.maxDepth(n)` is a safety ceiling (default 100), not a range.
- **There is no native `[1..d]` bounded-range operator** analogous to Cypher's
  `*1..d`. The docs explicitly punt: min/max-bounded variable-length
  traversal is deferred to an "Advanced patterns" page, and even that page's
  answer is `.times(n)` (exact depth) + `.emitAll()` (collect every
  intermediate hop) — i.e., the *union of all paths length 1..n*, which is a
  different (more expensive, more result-heavy) operation than Cypher's
  shortest-varlen semantics, or requires the caller to union multiple
  `.times(d)` queries manually.
- **Practical mapping for our harness:** to reproduce
  `MATCH (a {gid:$id})-[:LINK*1..d]->(b) RETURN b.gid LIMIT 1000` at a fixed
  target depth `d` (which is what P26/P30 actually measure — depths {1,3,6}
  are each a separate query, not a single *1..6* range query), the fair
  equivalent is `.repeat(sub().out("LINK")).times(d).valueMap(["gid"]).limit(1000)`
  run once per depth. This is method-comparable to how P26/P30 already treat
  depth (separate prepared queries per depth, not one range query) — **no
  methodology change needed**, just note in the writeup that HelixQL's
  `.repeat().times(d)` is exact-depth-d reachability (same semantic as our
  existing per-depth Cypher queries), not true `*1..d` union semantics, so
  cite it as "depth-d reachability" for both sides to keep it apples-to-apples.

### (c) Client bindings

Confirmed for all four: **Rust, TypeScript, Python, Go** — all are **HTTP
clients against `POST /v1/query`** on a running instance (local Docker
container or Helix Cloud), not embedded/in-process libraries.

- **Rust:** `helix_db::Client` is described in its own README as "a thin async
  wrapper over `reqwest`"; `Client::new(None)` defaults to
  `http://localhost:6969`; calls are `.await`-based (`client.query().dynamic(req).send().await?`).
  Tokio-based, async. Source: raw fetch of `sdks/rust/README.md`, 2026-07-03.
- **Python:** `Client("http://localhost:6969")` or `Client(local=True)`
  (via the separate `helix-py` package) with an `Instance` class for
  lifecycle-managed local instances. Sync-vs-async **not stated explicitly**
  in the fetched README content — marked **unknown**, needs a source read of
  `helix-py`'s client module before committing to a harness implementation
  language assumption (Python is likely simplest since it mirrors our
  `benches/*.py` pattern regardless of sync/async).
  Source: fetched README, 2026-07-03.
- **TypeScript, Go:** documented to exist (`@helix-db/helix-db` npm package;
  Go SDK setup doc page) but not independently fetched/verified — **unknown**
  sync/async status, not required for the harness plan below since Python is
  the intended implementation language (mirrors `ladybug_bench.py`/`kuzu_bench.py`/`neo4j_bench.py`).

---

## 3. Comparability hazards — must be stated explicitly in any published result

This inherits the **exact same shape as P23 (Neo4j)**, not P26/P30
(Kuzu/LadybugDB), and should be written up with the same "the whole point"
framing P23 used:

> GenesisBlockDB runs in-process; HelixDB is client-server behind Docker —
> each query pays an HTTP round-trip + JSON (de)serialization + (per HelixQL's
> builder model) AST-to-query compilation on the gateway, in addition to
> whatever the LMDB/LSM engine itself costs. Memory is GenesisBlockDB RSS vs.
> HelixDB container RSS (`docker stats`), which also includes the Rust HTTP
> gateway runtime and (if on-disk mode) a MinIO sidecar — not a clean
> engine-vs-engine memory comparison.

Concretely, three hazards to disclose, ranked by expected impact:

1. **Protocol tax dominates point/low-hop latency**, exactly as it did for
   Neo4j (P23: 120–185× gap at hop1, narrowing to 7–10× by hop6 as graph work
   starts to dominate over fixed per-query overhead). Expect a similar shape
   here: HelixDB's hop1 number will mostly measure HTTP+JSON+gateway
   overhead, not LMDB/graph traversal cost. Report hop1 as "server-tax
   dominated" explicitly, the way P23 did, rather than implying it reflects
   graph-engine speed.
2. **Ingest path asymmetry** (§2a): if `helix-py`'s loader is chunked HTTP
   inserts rather than a server-side bulk file load, GenesisBlockDB's WAL-durable
   ingest (which itself already trails LadybugDB's `COPY FROM` by ~48× per
   P30) may end up looking *artificially competitive* against HelixDB purely
   because HelixDB has no COPY-equivalent — that would be a hazard **in
   GenesisBlockDB's favor** and must be flagged just as prominently as hazards
   that favor the competitor, per this program's own house style (P30 already
   did this correctly for the Cypher-payload asymmetry).
3. **Memory accounting is container RSS, not process RSS.** `docker stats`
   reports cgroup memory for the whole `enterprise-dev` container (gateway +
   engine + any sidecar), which is not apples-to-apples with a Python
   `psutil.Process().memory_info().rss` read against a single embedded
   process. Report it as "HelixDB container RSS Δ" not "HelixDB engine RSS,"
   mirroring how P23 wrote "Neo4j JVM heap+store."

### Vendor benchmark numbers — why NOT directly usable against our P31 numbers

HelixDB's own published benchmark
([helix-db.com/blog/benchmarks](https://www.helix-db.com/blog/benchmarks),
fetched 2026-07-03, mirrored at
[docs.helix-db.com/benchmarks/v1](https://docs.helix-db.com/benchmarks/v1) —
**note: that docs-site URL 404'd on direct fetch during this scoping; the blog
URL is the one that resolved**):

| | Vendor bench | Our P31/P22/P26/P30 program |
|---|---|---|
| Hardware | AWS c6g.2xlarge — 8 vCPU **ARM Neoverse-N1**, 16 GB RAM, 500 GB gp3 EBS, eu-west-2 | Windows 10 Pro, **i7-8700K x86-64**, 32 GB RAM, C: SSD |
| HelixDB version tested | **v2.1.0** | n/a — v3.0.7 is current; version drift alone invalidates cross-referencing |
| Competitors in their bench | Neo4j 2025.09.0, PostgreSQL 16.10 (not GenesisBlockDB, not Kuzu/LadybugDB) | Neo4j (P23), Kuzu (P26), LadybugDB (P30), DuckDB+graph (P28), RocksDB+graph (P29) |
| Dataset | 10k users / 500k items / ~4M edges, workload-specific (PointGet/OneHop/OneHopFilter) | N=100k nodes, fanout-8 (~800k edges), depths {1,3,6} |
| Load model | FixedConcurrency (100–800) / FixedQPS (400–1600), 100-concurrent numbers quoted | Single-threaded sequential query loop, 200 q/depth |
| Reported P50 (100 concurrent) | PointGet 1.07 ms, OneHop 6.09 ms, OneHopFilter 2.94 ms | GenesisBlockDB hop1 21.6 µs, hop3 2.33 ms, hop6 4.40 ms (P30) |
| Authorship | **Vendor's own benchmark** (HelixDB testing itself vs. Neo4j/Postgres) | Third-party-style self-authored but methodology-transparent, reproducible scripts in `benches/` |

Reasons these numbers cannot be cross-referenced, all independently
disqualifying:

- **Different CPU architecture (ARM Neoverse-N1 vs. our x86-64 i7-8700K)** —
  no valid conversion factor exists; single-core IPC, cache hierarchy, and NUMA
  behavior all differ.
- **Different HelixDB version** (v2.1.0 vendor bench vs. v3.0.7 current) —
  engine internals may have changed materially between minor versions on a
  project this young (first public release May 2025 per the HN "Show HN"
  post).
- **Different competitor set** — their bench doesn't include GenesisBlockDB,
  Kuzu, or LadybugDB at all; there is no shared column to normalize against.
- **Different load model** — concurrent/QPS-driven load vs. our sequential
  single-connection loop measures fundamentally different things (queueing +
  concurrency scaling vs. raw per-query latency).
- **Vendor-authored** — not independently reproduced by a third party (see §5).

The only legitimate use of the vendor numbers is qualitative color ("HelixDB
claims parity-or-better vs. Neo4j on their own hardware/workload") — never a
quantitative delta against our own results.

---

## 4. Harness plan

### Design — mirrors `benches/ladybug_bench.py` exactly where the deployment model allows

Same topology as P26/P30: `N=100000`, fanout-8 independent random directed
edges, depths `{1,3,6}`, 200 queries/depth, `LIMIT 1000`. Differences forced
by HelixDB's architecture are called out inline.

- **File:** `benches/helix_bench.py` (Python, matching the existing
  `*_bench.py` convention — Python SDK is the natural choice given
  `helix-py`'s CSV loader and parity with `ladybug_bench.py`/`kuzu_bench.py`/`neo4j_bench.py`).
- **Setup (outside the timed script, like the Neo4j `docker run` in P23's
  reproduce block):**
  1. Install Docker Desktop (WSL2 backend) — one-time, not part of per-run
     timing.
  2. `curl -sSL "https://install.helix-db.com" | bash` (or fetch the
     `helix-x86_64-pc-windows-msvc.exe` release asset directly, since the
     install script is bash and this is a Windows host — **use the prebuilt
     `.exe` from the v3.0.7 release**, add to PATH).
  3. `helix init local --name gb-bench` (or CLI-equivalent project scaffold).
  4. `helix start dev` (or `--disk` if we want persistence parity with
     GenesisBlockDB's durable WAL — **recommend `--disk`** so the ingest
     comparison isn't "GenesisBlockDB durable WAL vs. HelixDB pure in-memory,"
     which would be an unstated hazard on top of the ones in §3). This pulls
     `ghcr.io/helixdb/enterprise-dev` + (if `--disk`) a MinIO sidecar.
  5. Confirm `http://localhost:6969` is reachable before starting the timed
     script.
- **Schema/setup (in HelixQL, via whichever SDK — Python client for
  parity):** define a `V` node type with a `gid` int property and a `LINK`
  edge type, analogous to LadybugDB's `CREATE NODE TABLE V(gid INT64, PRIMARY
  KEY(gid))` / `CREATE REL TABLE LINK(FROM V TO V)`. HelixQL schema syntax was
  not independently pulled in this scoping pass — **flag as a task-0 item in
  the harness build**, not a blocker (the querying-guide/data-model doc page
  exists at `docs.helix-db.com/database/data-model` and should be read first).
- **Ingest timing:** write the same `lb_nodes.csv` / `lb_edges.csv` shape
  (or reuse the files `ladybug_bench.py` already writes to
  `C:\Users\freshair\gb_vbench`), then bulk-load via `helix-py`'s
  `loader.py` CSV path. **Time this separately from schema setup**, and
  report it explicitly as "HTTP/loader-mediated ingest, no COPY-equivalent
  confirmed" per the §3 hazard #2 — do not present it as directly parallel to
  LadybugDB's `COPY FROM` number without that caveat inline in the same
  sentence.
- **Query timing:** for each depth `d` in `{1,3,6}`, build the HelixQL query
  `g().n(NodeRef.var("a")).where(id==sid).repeat(sub().out("LINK")).times(d).valueMap(["gid"]).limit(1000)`
  (exact builder syntax TBD against the Python SDK — see task list below),
  issue 200 queries with random `sid`, record wall-clock per call
  (`time.perf_counter()`, same as `ladybug_bench.py`), compute p50/p95/p99 in
  µs, write to `helix_results_<N>.json` in the same shape as
  `ladybug_results_<N>.json` for direct table reuse.
- **Memory:** `docker stats --no-stream <container>` parsed for RSS/mem-usage
  delta (base before ingest vs. after), reported as **container RSS**, not
  process RSS — label it that way in the results table per §3 hazard #3.
- **GenesisBlockDB side:** reuse P22/P26/P30's existing `graph-bench` Rust
  binary unchanged (`cargo run --release --bin graph-bench` with
  `GB_GRAPH_N=100000`) — no new work needed on our side.

### Effort estimate

| Task | Est. hours |
|---|---|
| Read HelixQL data-model + Python SDK docs closely enough to write real schema/query code (not just the builder sketch above) | 1.5 |
| Install Docker Desktop + WSL2 on the bench host, install `helix-cli`, smoke-test `helix start dev --disk` reachability | 1.0 |
| Write `benches/helix_bench.py`: CSV reuse, loader call, schema setup, per-depth query loop, JSON output matching `ladybug_bench.py` shape | 2.5 |
| Debug HelixQL builder syntax against a live instance (expect friction — this is a young, fast-moving DSL; `.repeat()`/`.times()` API surface may not match docs exactly) | 2.0 |
| Run at N=10k smoke, then N=100k full run; capture `docker stats` memory | 1.0 |
| Write the audit doc (mirroring `AUDIT--P23`/`P30` format) with the comparability caveats from §3 stated inline | 1.0 |
| **Total** | **~9 hours** (roughly one full working day; add ~2h contingency for Docker Desktop/WSL2 environment friction on this specific host, which is at the minimum supported Windows build) |

### Exact install commands (Windows 10)

```powershell
# 1. Docker Desktop (WSL2 backend) — manual download+install, requires reboot + admin
#    https://docs.docker.com/desktop/setup/install/windows-install/
#    Verify after install/reboot:
wsl --status
docker version

# 2. HelixDB CLI — use the prebuilt v3.0.7 Windows binary directly
#    (avoids piping the vendor's bash installer through Git Bash on a non-primary shell)
Invoke-WebRequest -Uri "https://github.com/HelixDB/helix-db/releases/download/v3.0.7/helix-x86_64-pc-windows-msvc.exe" -OutFile "$env:USERPROFILE\helix.exe"
# add to PATH or invoke by full path

# 3. Local project + instance
& "$env:USERPROFILE\helix.exe" init local --name gb-bench
& "$env:USERPROFILE\helix.exe" start dev --disk
# confirm:
curl http://localhost:6969/v1/query -Method POST -Body '{}' -ContentType "application/json"

# 4. Python SDK for the harness
pip install helix-py
```

---

## 5. Independent benchmarks of HelixDB v3 — search results

Searched: `"HelixDB benchmark 2026"`, `"HelixDB" review OR benchmark
site:reddit.com OR site:news.ycombinator.com 2026`, `HelixDB v3 vs Neo4j OR
Kuzu OR Qdrant independent comparison 2026` (all 2026-07-03).

**Finding: none found.** Every benchmark reference that surfaced traces back
to HelixDB's own blog/docs (`helix-db.com/blog/benchmarks`, testing v2.1.0 vs.
Neo4j/PostgreSQL, per §3) or third-party *marketing/summary* articles
(byteiota, ArcadeDB "Neo4j Alternatives in 2026", openalternative.co) that
repeat the vendor's claimed multipliers ("5–20x faster than Neo4j," "up to
1000x" in earlier launch material) without independent re-measurement. The
one Hacker News thread found is the original "Show HN" launch post from
2025-05-28 — discussion, not a benchmark. No independent reproduction of the
vendor's own `graph-vector-bench` repo
([github.com/helixdb/graph-vector-bench](https://github.com/helixdb/graph-vector-bench))
was found. This reinforces that a P34 head-to-head using our own methodology
would be the **first independent, reproducible number** for HelixDB v3
against an embedded competitor — genuinely novel, not duplicating existing
work.

---

## 6. Sources (all fetched/searched 2026-07-03)

- [github.com/HelixDB/helix-db](https://github.com/HelixDB/helix-db) — repo root, contents, LICENSE, Cargo.toml (GitHub API + WebFetch)
- [github.com/HelixDB/helix-db/releases/tag/v3.0.7](https://github.com/HelixDB/helix-db/releases/tag/v3.0.7) — release body + assets (GitHub API)
- `helix-cli/src/local_runtime.rs` (raw.githubusercontent.com, main branch) — Docker/Podman launch logic
- `sdks/rust/README.md`, `sdks/python/README.md` (raw.githubusercontent.com, main branch) — client architecture
- [docs.helix-db.com](https://docs.helix-db.com/) — intro, `llms.txt` index, `/database/local-development`, `/database/querying-guide/traversals`, `/database/querying-guide/advanced`, `/cli/getting-started`
- [docs.helix-db.com/benchmarks/v1](https://docs.helix-db.com/benchmarks/v1) — **404 at fetch time**, use blog URL instead
- [helix-db.com/blog/benchmarks](https://www.helix-db.com/blog/benchmarks) — vendor benchmark methodology + numbers
- [helix-db.com/blog/helix-cli-docker-deploy](https://www.helix-db.com/blog/helix-cli-docker-deploy) — Docker deploy flow
- [github.com/HelixDB/helix-py](https://github.com/HelixDB/helix-py) — Python SDK + `loader.py`
- [deepwiki.com/HelixDB/helix-db](https://deepwiki.com/HelixDB/helix-db) — architecture summary (secondary source, cross-checked against primary repo evidence)
- [dbdb.io/db/helixdb](https://dbdb.io/db/helixdb) — cross-checked, found stale license claim (GPLv3, contradicted by primary-source LICENSE file — Apache-2.0 is correct)
- [news.ycombinator.com/item?id=43975423](https://news.ycombinator.com/item?id=43975423) — original Show HN launch thread (2025-05-28)
- Docker Desktop Windows/WSL2 requirements — docs.docker.com/desktop/setup/install/windows-install/ (WebSearch synthesis, fetched 2026-07-03)
- This repo: `docs/AUDIT--P23-NEO4J-HEAD-TO-HEAD.md`, `docs/AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD.md`, `benches/ladybug_bench.py` (local read, for method precedent)
