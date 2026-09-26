# `@freshair129/gks-genesis-block-native-linux-x64-gnu`

This is the **x86_64-unknown-linux-gnu** binary for `@freshair129/gks-genesis-block-native`

## The committed `index.linux-x64-gnu.node` (GenesisRAG17 worker, ADR-075 Phase 3 / P-2)

`index.linux-x64-gnu.node` in this directory is committed **on purpose**, and it
is a deliberate exception to the rule in `.gitignore` that per-platform `.node`
files are CI-built and distributed through the per-triple optional-dependency
packages rather than checked in. `.gitignore` carries a single negation for this
one path and nothing else; do not widen it to other triples.

The exception exists for one consumer: the **GenesisRAG17 Tier 4 worker**
(`genesisrag17-worker/`) needs a Linux addon at a *pinned* commit so the zuri-ai
edge image's `ki17` build stage can copy a known-good artifact instead of
compiling Rust inside that image. This is **not** part of the general
GenesisBlockDB npm distribution tranche; nothing here changes how the published
per-triple packages get their binaries.

| Fact | Value |
|---|---|
| Built from | GenesisBlock `907b0ff4ca66e5940741bfa284e92c157dc783f7` — the merge of PR #174, the ADR-075 Phase 2 "accept both ontology versions" worker change |
| Target triple | `x86_64-unknown-linux-gnu` |
| Build command | `npm ci --ignore-scripts && npm run build` (`napi build --platform --release`) |
| Build environment | A throwaway Docker container from the official `rust:1-bookworm` image (`rustc 1.98.1`), source mounted read-only, artifact written out to the host. No `docker compose` was involved |
| Size | 9,987,776 bytes (9.5 MiB), stripped |
| SHA-256 | `ff28bcf5214090b9ce4b8ec8ca6b69fd134e0b2fd3115d5df78db29d24191696` |
| Node used to verify | `v24.18.0` linux-x64 — what `genesisrag17-worker/package.json` requires (`"node": ">=24.18"`) and what the MSP GenesisRAG17 runbook pins |
| Verified with | `npm test --prefix genesisrag17-worker` on Linux, in a second container, against this exact file: **20 tests, 14 passed, 0 failed, 6 skipped** |

### Runtime requirements of this file

`ldd` reports only `libgcc_s.so.1`, `libm.so.6` and `libc.so.6` — no
`libstdc++`, no OpenSSL, no system SQLite (`rusqlite` is bundled). The highest
versioned symbol it imports is **`GLIBC_2.34`**, so:

- Debian **bookworm** (glibc 2.36) — including `node:22-bookworm-slim`, the base
  of the zuri-ai web image — loads it. ✅
- Debian bullseye (glibc 2.31) does **not**. ❌
- **musl (Alpine) does not, at all.** This is a glibc build; the triple says so.

### Why the model-gated tests skip, and what that leaves unproven

The pinned `intfloat/multilingual-e5-small` snapshot (revision
`614241f622f53c4eeff9890bdc4f31cfecc418b3`, ~490 MB) is not provisioned inside a
build container. `worker.test.mjs` detects the missing snapshot and reports the
six tests that need it as *skipped* rather than failed, the same way
`test.yml`'s `worker-tests` job already does on all three hosts. The six are:

- worker verifies pinned model artifacts and performs native publish/query
- worker resumes after pointer replacement without rewriting the snapshot
- worker keeps a prepared snapshot private until pointer replacement
- worker reuses an actual native final transaction after commit-to-receipt interruption and indexes lexical rows only in Stage16
- worker checkpoints a new vector collection before a crash can replay its first vector transaction
- worker publishes two queued documents and retains versioned historical snapshots after correction

So the Stage 15 embedding and Stage 16 lexical-index paths have **not** been
exercised on Linux by this artifact's verification run. The 14 that did pass
cover the MSP mock boundary, Stage 13 graph commits with both `ontology_v1` and
`ontology_v2`, durable native transaction intents and replay, receipt recovery,
atomic pointer replacement and its failure mode, WARN fail-closed publication,
and the C-10 bitemporal lane count.

Before this file existed, every GenesisRAG17 acceptance run had been on Windows
x64 and the only addon anyone had built at the pin was
`index.win32-x64-msvc.node` — constraint **C11** of the zuri-ai deployment design
`docs/plans/GENESISRAG17-EDGE-DEPLOYMENT.md`. This artifact is prerequisite
**P-2** of that design's §8. It does **not** satisfy gate **G-3**, which still
requires a full acceptance run on Linux with the model snapshot present.

### Keeping it from drifting

`.github/workflows/genesisrag17-worker-linux.yml` runs on every push and pull
request and exercises the worker suite twice: once against this committed file
(proving the artifact a consumer would copy still loads and passes) and once
against a fresh `napi build` for the same triple (proving the source still
produces a working Linux addon).

It deliberately does **not** compare the two byte-for-byte. A different `rustc`
patch release, build path or LTO run changes the bytes without changing the
behaviour, and a check that goes red for that reason is a check that gets
ignored.
