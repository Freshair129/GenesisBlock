# Engine Repo Product-Surface Audit — G:\GenesisBlock_Dev\GenesisBlock

Skeptical due-diligence pass, code evidence only. Date: 2026-07-07.

## 1. Engine product readiness (npm)

**Not published. A stranger cannot `npm install` this today.**

- Package name is NOT `genesis-block-native` — it is `@freshair129/gks-genesis-block-native`, version `0.2.0` (`package.json:2-3`).
- **npm registry check (live, this session):** `https://registry.npmjs.org/@freshair129%2Fgks-genesis-block-native` returns `{"error":"Not found"}` — never published under any version. All four `optionalDependencies` platform packages (`package.json:54-59`) are equally unpublished, so even a manual tarball install would fail to resolve prebuilds.
- Publish machinery EXISTS but has never fired for 0.2.0: `.github/workflows/release.yml` is a complete 4-target napi matrix (linux-x64-gnu, win32-msvc, darwin-x64/arm64) + `npm publish --access public --tag beta` gated on tag push and a repo secret `NPM_TOKEN` (release.yml:6, comment at top says "PREREQUISITES (must be set up once before this can actually publish)"). Git tags: only `v0.1.0-beta.1` and `v0.1.0-beta.2` exist — **no `v0.2.0` tag**, so the workflow has never run for the current version. Also note `npm publish --tag beta` is hardcoded (release.yml:110), wrong for a non-beta 0.2.0.
- Consumer docs exist and are decent: `README.md` (quickstart links, measured perf), `QUICKSTART.md` ("5-minute Quickstart (Node.js)"). **But QUICKSTART.md is stale/wrong on install**: it says "`npm install` compiles the Rust native addon (`napi build`), so a Rust toolchain must be on PATH" — there is no `install`/`prepare` script in `package.json:36-48` (CLAUDE.md confirms this was removed by design). A stranger following QUICKSTART would `npm install` a nonexistent package and then be told they need cargo when they don't.
- `CHANGELOG.md` is real and maintained (0.2.0 entry dated 2026-06-29, Unreleased section for HQL MATCH patterns). `SECURITY.md` exists but is **stale**: supported-versions table still says `0.1.0-beta.x` supported (`SECURITY.md:8-10`) while shipped version is 0.2.0.
- Other CI: `test.yml`, `security.yml`, `mobile-build.yml`, `benchmarks.yml`, `pages.yml` all present in `.github/workflows/`.

**Blockers for a stranger today:** (1) package unpublished, (2) no v0.2.0 tag / release run, (3) NPM_TOKEN secret setup unverified (workflow comment says "must be set up once" — UNVERIFIED whether done), (4) QUICKSTART install instructions contradict actual package.json, (5) beta dist-tag hardcoded.

## 2. Python SDK (`genesisdb-python/`) — what a Graphiti driver would build on

**Prototype-grade, ~134 lines total, unpublished.** Grade: D+ (works as a demo, not a dependency).

- Contents: `setup.py` (12 lines, name `genesisdb` v0.1.0, only dep `requests`), `genesisdb/client.py` (88 lines), `models.py` (30 lines, 3 dataclasses), `exceptions.py` (11 lines), `examples/basic_usage.py`. That is everything.
- **NOT on PyPI**: `https://pypi.org/pypi/genesisdb/json` → `{"message": "Not Found"}` (live check). Name `genesisdb` may also be squattable/taken-risk — UNVERIFIED who owns it.
- **No README, no tests, no pyproject.toml, no CI job** (verified by directory listing; only setup.py + package + examples).
- It is a thin **REST** client, not NAPI: hardcoded default `http://localhost:3000` (`client.py:8`), three methods only — `query()` → POST `/v1/query/hql` (client.py:22), `add_node()` → POST `/v1/node/add` (client.py:38), `get_context()` which just builds a `CONTEXT FOR ... TIER ...` HQL string and calls `query()` (client.py:71). `_check_connection` is a **no-op with a comment admitting it** ("We don't have a specific health endpoint yet... just assume alive", client.py:12-18 — false, `/v1/status` exists at router.rs:648).
- No auth support: router.rs applies an `api_key_guard` middleware (router.rs:660) but the Python client sends no key header — it can only talk to an unguarded server. Same for Go.
- `genesisdb-go/` is the same shape: `client.go` (114 lines: Query/AddNode/GetContext over REST), `models.go` (40 lines), one example, `go.mod` module path `github.com/freshair129/genesisblock-go` (go.mod:1) — which does not match the directory name `genesisdb-go`; no README, no tests, not tagged for `go get` (UNVERIFIED whether the GitHub path exists publicly).

## 3. MCP server (`mcp/server.js`, 144 lines)

- **Tools (exact, verified):** `query_hql`, `retrieve_tiered_context`, `add_knowledge` — that's the full `TOOLS` array (server.js:36-77). Matches CLAUDE.md's claim.
- Transport: **stdio only** (`StdioServerTransport`, server.js:135).
- Binding: consumes the **NAPI addon in-process** (`require("../index.js")` → `GenesisDatabase.open`, server.js:8,15) — NOT the REST server. So MCP requires a built `.node` binary; a stranger must clone + `npm run build` (Rust toolchain) first. `docs/MCP-GUIDE.md:12-16` confirms: install = `npm install; npm run build`.
- Hardcoded config: DB at `.brain/mcp_db` (env-overridable, server.js:12), `vectorDim: 1536` hardcoded (server.js:19) — OpenAI-dim default, mismatched with the bge-m3/1024 used everywhere in benchmarks; `causedBy: "mcp-agent"` and `lang: "en"` hardcoded in add_knowledge (server.js:110-111).
- Install story: `npm run mcp:start` script only (package.json:47). No npx-able published package, no `bin` entry, no `.mcp.json` at repo root.
- Consumers today: repo evidence only — `__test__/mcp.test.mjs` (own tests). Per memory, NotiKeeper built its **own** mcp-server rather than using this one; SDD-Integration (quoted in `docs/genesis-interview/evidence/ground-govibe.md:140`) plans GKS/GoVibe to use "dual surface NAPI fast-path + MCP". No production consumer of `mcp/server.js` found in-repo.

## 4. Shared-surface analysis: wedge vs platform

**Wedge consumers (hypothetical Graphiti GraphDriver / LangGraph checkpointer, both Python):**
- Would consume the **REST surface** (`/v1/*`) via the Python SDK or raw `requests` — Python cannot load the NAPI cdylib, and there are no Python native bindings (no PyO3 anywhere; `src/ffi.rs` C ABI is mobile-gated, 8 symbols, not a Python wheel). So the wedge stack is: **Rust core (`src/lib.rs`) → Axum REST (`src/router.rs`, `src/main.rs`) → genesisdb-python (or a new driver-specific client)**.
- Concretely a Graphiti driver's `execute_query` maps to POST `/v1/query/hql` (router.rs:627-630) + `/v1/node/add`, `/v1/edge/add`, `/v1/search/hybrid` (router.rs:631-646). A LangGraph checkpointer's put/get maps to `/v1/node/add` + `/v1/query` / node lookup. Both are REST-only.
- Caveat: REST requires running `genesis-db-server` (`cargo run --features bins`), which is **not shipped as a binary anywhere** — no Docker image, no release artifact for the server (release.yml builds only the .node addon). That is a real wedge blocker.

**Platform consumers (GKS/MSP/GoVibe/Rwang):**
- Per SDD-Integration as quoted in `docs/genesis-interview/evidence/ground-govibe.md:140`: "dual surface NAPI fast-path + MCP is required for latency... GenesisBlockDB already ships both". Plus governance tiers "enforced in GenesisBlockDB engine". So platform = **Rust core → NAPI addon (`GenesisDatabase`) + mcp/server.js (which itself sits on NAPI)**. The npm package name itself says it: "@freshair129/**gks**-genesis-block-native... for GKS (P3.1 scaffold)" (package.json:2-4).

**Explicit overlap:**
- **Shared by BOTH:** the entire Rust core — storage/WAL, HNSW, graph indices, bitemporal model, **HQL pipeline** (`src/query/hql.pest` + `ast.rs` + `execute_hql`), governance, GRL/CONTEXT. This is ~6,000 lines of lib.rs + src/query/, i.e. the overwhelming majority of the codebase. Both paths end at the same `Storage` methods.
- **Wedge-only:** REST server (`src/main.rs`, `src/router.rs` ~680 lines), genesisdb-python (~134 lines), genesisdb-go (~154 lines), plus the to-be-written driver adapters (~hundreds of lines each, new code). Wedge-only code that exists today ≈ under 1,000 lines vs a ~6k-line shared core.
- **Platform-only:** NAPI async wrapper (`GenesisDatabase` in lib.rs, napi-bindings feature), `mcp/server.js` (144 lines), plus everything living outside this repo (Rwang/GoVibe/GKS/MSP).
- **Notable asymmetry:** `events_since` (CRDT event feed, lib.rs:5393 core / lib.rs:5959 NAPI) is exposed on **NAPI only** — grep of router.rs finds no `/v1/sync` or `/v1/events` route. A Graphiti driver wanting incremental sync, or any wedge-side change-feed, would need this REST exposure; the platform's shadow-sync already uses it internally (lib.rs:4448).

## 5. Verdict

**Yes, the wedge is on the platform's critical path — shared code is the core itself.** By line count the shared engine (lib.rs + src/query) dwarfs both binding layers; wedge-only code today is <1k lines of thin HTTP glue. Nothing built for a Graphiti/LangGraph driver at the engine level is wasted for GKS/MSP, because both terminate in the same `Storage` methods.

Engine work items that serve BOTH (with evidence they're already demanded by the platform side):
1. **HQL v-next (P0 defects + var-length paths):** `docs/genesis-interview/evidence/ground-govibe.md:152` states the platform's own `query_genesis_graph(hops=N)` can't express bounded var-length paths (P1 in `docs/PLAN--HQL-REFINEMENT.md`); a Graphiti driver's `execute_query` needs the same expressiveness. ground-govibe.md:43: bitemporal "gap is query-surface, not storage" — dual-axis AS OF serves Graphiti (which is natively bitemporal) directly.
2. **`events_since` REST exposure:** NAPI-only today (router.rs has no events/sync route); needed by any wedge sync/replication story and already consumed internally by platform shadow-sync.
3. **Bitemporal audit/supersede-chain enumeration** (ground-govibe.md:43): demanded by the MSP data model AND is exactly what differentiates a Graphiti backend driver.
4. **Publish plumbing** (npm publish of 0.2.0, PyPI publish, a shippable server binary/Docker image): pure wedge-enablement, zero platform cost, and currently the single largest gap — the engine is at "stranger must clone + install Rust + build" for every surface except none.

**What is NOT shared:** distribution work per channel (Python driver code, REST hardening/auth in SDK clients, server packaging) is wedge-specific; MCP polish is platform-specific. But these are all thin layers over the same core.
