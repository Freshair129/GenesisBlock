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
| Built from | PR #217 merge checkout `209cc36b455c6e9622b046eb6060b69d9cd4d681` (hosted workflow run `37439589499`, job `112189748409`); this is build-source provenance, not the later distribution commit |
| Engine qualification | Candidate upgrade; historical integration baseline `e15e35b0093394e0a8880af7f4e6f63cf81223b7` remains historical evidence only. See the [worker ADR](../../docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md) |
| Target triple | `x86_64-unknown-linux-gnu` |
| Build command | `npm ci --ignore-scripts && npm run build -- --cargo-flags=--locked` (`napi build --platform --release`) |
| Build environment | Debian Bookworm container `rust:1.98.1-bookworm`; `rustc 1.98.1 (48a229cea 2026-09-01)`, Cargo `1.98.1`, Node `v24.18.0`, npm `11.16.0`; runner Ubuntu 24.04.5 |
| Cargo.lock SHA-256 | `b58d8a0490cf44c88b905d573e2f7cb51d1fd8bc7d61ea061ad026a6d7eb709e` |
| package-lock.json SHA-256 | `4dccc43c6ae07e7fb840b52342a65be2c47498308cae29396c1abf3e1ba0eb70` |
| Size | 11,039,392 bytes (10.5 MiB) |
| SHA-256 | `b678dfd7ee125d33d877b008e3af2c2a36ee9fa5203d043248cf2b9974e778e1` |
| Highest imported GLIBC | `GLIBC_2.34` |
| Node used to verify | `v24.18.0` linux-x64 — what `genesisrag17-worker/package.json` requires (`"node": ">=24.18"`) and what the MSP GenesisRAG17 runbook pins |
| Verified with | Freshly built candidate in run `37439589499`: **32 tests, 26 passed, 0 failed, 6 skipped** because the pinned ONNX model snapshot was absent; the pre-refresh committed addon failed 3 recovery cases. The refreshed committed addon (this file's SHA-256 above) passed the same **26/0/6** suite on PR head `ab38d5079510ccaf566a180bf677a0e94a38bbea`, run `37443730184`, job `112203403569`. The six model-dependent tests remain unverified. |

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
exercised on Linux by this artifact's verification run. The 26 passing tests
include checks for the MSP mock boundary, Stage 13 graph commits with both
`ontology_v1` and `ontology_v2`, durable native transaction intents and replay,
receipt recovery, atomic pointer replacement and its failure mode, WARN
fail-closed publication, and the C-10 bitemporal lane count.

Before this file existed, every GenesisRAG17 acceptance run had been on Windows
x64 and the only addon anyone had built at the pin was
`index.win32-x64-msvc.node` — constraint **C11** of the zuri-ai deployment design
`docs/plans/GENESISRAG17-EDGE-DEPLOYMENT.md`. This artifact is prerequisite
**P-2** of that design's §8. It does **not** satisfy gate **G-3**, which still
requires a full acceptance run on Linux with the model snapshot present.

### Keeping it from drifting

`.github/workflows/genesisrag17-worker-linux.yml` runs on every push and pull
request. It first builds from current source with Cargo's lockfile enforced,
tests the rebuilt addon, and uploads that tested output. It then restores the
committed wrappers and addon and runs the same worker suite against the
committed file a consumer would copy; the restored binary's hash must match
the value captured at job start.

It deliberately does **not** compare the two byte-for-byte. A different `rustc`
patch release, build path or LTO run changes the bytes without changing the
behaviour, and a check that goes red for that reason is a check that gets
ignored.
