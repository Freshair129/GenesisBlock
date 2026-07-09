# 33 — Task Breakdown (Waves W1–W4)

> **Machine SSOT:** `queue/IMPLEMENTATION_QUEUE.json`. Titles/status here MUST match; JSON is authoritative (RWANG §8).
> **Local dispatch:** every task carries `LOCAL_SAFE` or `CLOUD_REQUIRED` (§12.4). The dispatcher (Rwang / manual) reads the JSON.
> **Frozen constraints (all tasks):** no changes to Storage core methods, public NAPI/REST/FFI signatures, on-disk/WAL format, or HQL grammar semantics. Any such change requires `docs/ARCHITECTURE_CHANGE_REQUEST.md`.

---

## Wave W1 — Publish Engine (unblocks GATE-DEMAND-1)

### TASK-0001 — Verify + dry-run release pipeline
- **Category:** Infrastructure · **Complexity:** S · **Context:** Small · **Local:** CLOUD_REQUIRED (secrets + CI) · **Verification:** Build · **Deps:** none · **Ready:** ✅
- **Purpose:** confirm `NPM_TOKEN` valid, GitHub release workflow builds all 5 target triples (napi artifacts), and prebuild optionalDependency packages resolve.
- **Scope in:** GitHub Actions dry-run; secrets check; matrix output inspection. **Out:** publishing (TASK-0004).
- **Outputs:** dry-run log; go/no-go note in `state/events.jsonl`.
- **Acceptance:** matrix green for all 5 triples; `NPM_TOKEN` proven to have publish rights (whoami OK).
- **Risks:** token expired / missing; matrix broken since last touch.

### TASK-0002 — Fix publish dist-tag
- **Category:** Infrastructure · **Complexity:** XS · **Context:** Tiny · **Local:** LOCAL_SAFE · **Verification:** StaticAnalysis · **Deps:** none · **Ready:** ✅
- **Purpose:** hardcoded `npm publish --tag beta` in `package.json`/scripts is wrong for a `0.2.0` stable release.
- **Scope in:** `package.json` scripts. **Out:** actual publish.
- **Acceptance:** publish command chooses `latest` for x.y.z, `beta`/`rc` only when semver has prerelease.

### TASK-0003 — Fix QUICKSTART + SECURITY.md
- **Category:** Documentation · **Complexity:** XS · **Context:** Small · **Local:** LOCAL_SAFE · **Verification:** StaticAnalysis · **Deps:** none · **Ready:** ✅
- **Purpose:** QUICKSTART's install steps contradict `package.json` (no `prepare` script); SECURITY.md still lists 0.1.0-beta.
- **Outputs:** updated `QUICKSTART.md`, `SECURITY.md`.
- **Acceptance:** copy-paste QUICKSTART commands on a clean machine actually works.

### TASK-0004 — Tag v0.2.0 + run release matrix + publish
- **Category:** Infrastructure · **Complexity:** M · **Context:** Small · **Local:** CLOUD_REQUIRED (publish + human confirm) · **Verification:** Build · **Deps:** TASK-0001..3 · **Ready:** blocked
- **Purpose:** cut `v0.2.0`, produce prebuild binaries, publish `optionalDependencies` per-triple + main package.
- **Acceptance:** `npm view @freshair129/gks-genesis-block-native versions` includes `0.2.0`; all 5 platform packages present.

### TASK-0005 — Clean-machine smoke
- **Category:** Testing · **Complexity:** S · **Context:** Small · **Local:** LOCAL_SAFE · **Verification:** Unit · **Deps:** TASK-0004 · **Ready:** blocked
- **Purpose:** proof that a stranger's `npm install` → load `.node` → add/search round-trip works (no G:/ paths anywhere).
- **Acceptance:** fresh temp dir + `npm i` + 20-line node script passes.

---

## GATE-DEMAND-1 (after W1)
Owner-authority gate: **first-10-external-installs signal within ~1 month** of publish. Recorded in `state/events.jsonl`. Blocks W2/W3/W4 until decision.

---

## Wave W2 — Adapter + MCP (gated)

### TASK-0006 — Extract Rwang adapter → path-independent
- **Category:** Core · **Complexity:** M · **Context:** Medium · **Local:** LOCAL_SAFE · **Verification:** Unit · **Deps:** TASK-0004
- **Purpose:** lift `store/genesis-sidecar.mjs` + `store/knowledge.mjs` out of Rwang; remove `G:/` default; take DB path via config/param only. This becomes the first published, path-independent consumer contract.
- **Frozen:** engine surface unchanged; NAPI signatures unchanged.
- **Acceptance:** loads engine binding from `optionalDependencies` (no path guess); addNode/hybridSearch/retrieveContext/GRL wrappers unit-tested; zero references to `G:/` or `G-Maiden`.

### TASK-0007 — Package + publish adapter
- **Complexity:** S · **Local:** CLOUD_REQUIRED (publish) · **Deps:** TASK-0006
- **Acceptance:** package name published; README shows a 10-line install-and-use snippet.

### TASK-0008 — Make `mcp/server.js` npx-able
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Snapshot · **Deps:** TASK-0004
- **Purpose:** `bin` entry in `package.json`; published; `npx @freshair129/gks-mcp` starts an MCP memory server against a local DB path.
- **Acceptance:** stdio handshake with an MCP client works.

### TASK-0009 — Fix MCP vectorDim 1536 → 1024
- **Complexity:** XS · **Local:** LOCAL_SAFE · **Verification:** Unit · **Ready:** ✅
- **Purpose:** hardcoded `vectorDim: 1536` contradicts bge-m3/1024 default used everywhere else.
- **Acceptance:** default matches engine collection dim; overridable via env/config; test proves it.

### TASK-0010 — Adapter contract tests
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Unit · **Deps:** TASK-0006
- **Acceptance:** path-independent load + round-trip on a temp-dir DB.

---

## Wave W3 — REST binary + Docker + SDK auth (gated)

### TASK-0011 — Dockerfile
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Build · **Ready:** ✅
- **Purpose:** multi-stage Dockerfile building `genesis-db-server` with `--features bins`, small runtime image.
- **Acceptance:** `docker build` + `docker run -p 3000:3000` responds on `/v1/status`.

### TASK-0012 — SIGTERM graceful shutdown → save_state()
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Compiler · **Ready:** ✅
- **Purpose:** `src/main.rs:53` has no shutdown hook; ctrl-c / docker stop must call `save_state()` before exit.

### TASK-0013 — Unauthenticated `/health` route
- **Complexity:** XS · **Local:** CLOUD_REQUIRED (public interface add) · **Verification:** Unit · **Ready:** ✅
- **Purpose:** orchestrator probes need `/health` without the key guard (today `/v1/status` requires the API key).
- **Acceptance:** 200 without header; distinct from `/v1/status`.

### TASK-0014 — SDK auth header support
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Unit · **Ready:** ✅
- **Purpose:** Python + Go SDKs must send `Authorization: Bearer <GENESIS_API_KEY>` matching `api_key_guard`.

### TASK-0015 — REST API contract tests incl. auth path
- **Complexity:** M · **Local:** LOCAL_SAFE · **Verification:** Unit · **Deps:** TASK-0013, TASK-0014
- **Acceptance:** existing 20-route contract suite extended with `/health` (no auth) + authed CRUD via SDK.

---

## Wave W4 — Graphiti driver (gated + contract-check-gated)

### TASK-0016 — Graphiti GraphDriver contract analysis
- **Category:** Documentation · **Complexity:** M · **Context:** Large · **Local:** CLOUD_REQUIRED (architectural decision) · **Verification:** Review
- **Purpose:** read Graphiti's `GraphDriver` interface; map each required method to an HQL/REST call; output an adapter-or-translator verdict.
- **Outputs:** `docs/adr/ADR--GRAPHITI-DRIVER-CONTRACT.md` (adapter | translator | infeasible).
- **Acceptance:** every required method has a concrete implementation strategy OR is documented as a gap; verdict is unambiguous.

### TASK-0017 — Implement Graphiti driver
- **Complexity:** L · **Local:** CLOUD_REQUIRED (public integration) · **Verification:** Unit · **Deps:** TASK-0016 (verdict = adapter), TASK-0011, TASK-0014
- **Acceptance:** all Graphiti driver methods pass their upstream test-shape.

### TASK-0018 — Conformance tests + example
- **Complexity:** S · **Local:** LOCAL_SAFE · **Verification:** Snapshot · **Deps:** TASK-0017
- **Acceptance:** a runnable example under `examples/graphiti/` + conformance suite.

---

## Local-dispatch summary (§12.4)

| Wave | Total | LOCAL_SAFE | CLOUD_REQUIRED |
|---|---|---|---|
| W1 | 5 | 3 (0002, 0003, 0005) | 2 (0001, 0004) |
| W2 | 5 | 4 (0006, 0008, 0009, 0010) | 1 (0007) |
| W3 | 5 | 4 (0011, 0012, 0014, 0015) | 1 (0013) |
| W4 | 3 | 1 (0018) | 2 (0016, 0017) |
| **Total** | **18** | **12 (67%)** | **6 (33%)** |

Ready-now for local pickup (deps=[], ready=true, LOCAL_SAFE): **TASK-0002, TASK-0003, TASK-0009, TASK-0011, TASK-0012, TASK-0014.**
