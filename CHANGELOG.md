# Changelog

All notable changes to GenesisBlockDB are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — npm provenance rejected the main native-addon publish: missing `repository.url`

Found by actually pushing the `v0.2.2` tag: with `NPM_TOKEN` set and the
`napi.triples.additional` fix (0.2.2) in place, `napi artifacts` succeeded
for the first time ever, but the subsequent `npm publish` step failed:

```
npm error code E422
npm error 422 Unprocessable Entity - PUT .../@freshair129%2fgks-genesis-block-native-win32-x64-msvc
npm error Error verifying sigstore provenance bundle: Failed to validate
repository information: package.json: "repository.url" is "", expected to
match "https://github.com/Freshair129/GenesisBlock" from provenance
```

Root cause: root `package.json` had no top-level `repository` field at
all. `release.yml`'s publish step runs with npm provenance enabled (`npm
config set provenance true`), which cryptographically ties the publish to
this exact GitHub repo/workflow run — npm's registry then requires
`package.json`'s `repository.url` to match, and an empty/missing field
fails verification outright. `react-native-genesisdb/package.json` already
had this field (its own publish succeeded cleanly); the root package.json
was the one surface missing it.

- Added a `repository: { type: "git", url: "https://github.com/Freshair129/GenesisBlock" }`
  field to `package.json`.
- Re-ran `napi create-npm-dir -t .` to regenerate the committed
  `npm/{linux-x64-gnu,win32-x64-msvc,darwin-x64,darwin-arm64}/package.json`
  skeletons (added in 0.2.2's `napi.triples.additional` fix) so they pick up
  the `repository` field too — `napi`'s own scaffolder copies `repository`
  from the root package.json into each per-platform package. Also
  incidentally caught these skeletons up from a stale `0.2.1` to the
  current `0.2.2` version string.

Separately, not a bug: the same `v0.2.2` tag's `android-publish` job failed
with a `409 Conflict` — expected, not a defect. `genesisdb-android:0.1.0`
was already published successfully during the `v0.2.1` run and is still
live; GitHub Packages correctly rejects re-publishing an unchanged version
at the same coordinate. Nothing to fix there.

## [0.2.2] - 2026-08-25

Patch release — no engine/runtime code changed since v0.2.1. Cuts a real
release tag now that the `NPM_TOKEN` repo secret (the last missing
prerequisite from `release.yml`'s own header comment) has been added, so the
`v0.2.1` npm-publish attempts that failed with `ENEEDAUTH` can finally
succeed for real: the main native-addon package, `genesisdb-android` (already
live on GitHub Packages since v0.2.1 and expected to re-publish harmlessly),
and `react-native-genesisdb`.

### Fixed — `napi artifacts` silently dropped Apple Silicon macOS from every npm release

Found by actually pushing the `v0.2.1` tag and watching `release.yml`'s `publish` job run for real for the first time ever (every prior tag either got stuck on a dead `macos-13` runner or was cancelled before reaching this job) — it failed with `TypeError: No dist dir found for .../bindings-aarch64-apple-darwin/index.darwin-arm64.node`.

Root cause: `package.json`'s `napi` config declared `aarch64-apple-darwin` under `targets` (used for cross-compilation), but napi-rs's packaging tooling (`napi create-npm-dir`, `napi artifacts`) doesn't read that key at all — it reads `napi.triples.additional`, which was never set. Without it, only the three napi-rs *default* platforms (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`) got a per-platform npm package; Apple Silicon macOS was silently excluded from `napi create-npm-dir`'s output, so `napi artifacts` had nowhere to write the arm64 macOS binary and threw. `optionalDependencies` in the root `package.json` already listed a `-darwin-arm64` package (pinned to a stale `0.2.0`, expecting this to work) — so this was an unintentional regression/oversight, not a deliberate exclusion.

- Added `napi.triples.additional: ["aarch64-apple-darwin"]` to `package.json`.
- Ran `napi create-npm-dir -t .` and committed the resulting `npm/{darwin-arm64,darwin-x64,linux-x64-gnu,win32-x64-msvc}/{package.json,README.md}` skeletons — these are the standard napi-rs per-platform package placeholders CI fills the compiled `.node` binary into at publish time; they had never existed in this repo at all before.
- Verified the fix locally: simulated the full `napi artifacts` step with dummy per-platform `.node` files matching the real CI artifact layout — confirmed all 4 platforms now resolve and write correctly (previously 3 succeeded silently and the 4th threw, aborting the whole step).

Not fixed here (separate, non-blocking, no behavioral impact): `napi.binaryName: "genesis-block"` in `package.json` is also not a key napi-rs's tooling reads (it reads `napi.name`, defaulting to `"index"` when absent) — every actual build artifact is already consistently named `index.<platform>.node` on both the build and packaging sides, so there's no naming mismatch today, just a misleading/dead config key. Left alone to keep this fix minimal; renaming would touch the binary file name itself and `index.js`'s loader.

## [0.2.1] - 2026-08-24

Patch release — no engine/runtime code changed. Cuts a real release tag so
the mobile SDK publish-infrastructure work (issue #125) and the two docs-only
release-asset PRs actually run through the release pipeline: `v0.2.0` was
already tagged (pointing at an earlier commit) before this infrastructure
existed, and tags don't move, so a new patch version is the correct way to
get CI to build+publish with today's `release.yml`.

### Added — mobile SDK package-manager publishing infrastructure (issue #125)

- **`genesisdb-android` → GitHub Packages**: `android/genesisdb/build.gradle.kts`
  gained a `maven-publish`-backed `publishing {}` block (AGP `singleVariant`
  release component + sources jar) publishing `dev.genesisblock:genesisdb-android:0.1.0`
  to `https://maven.pkg.github.com/Freshair129/GenesisBlock`. Chosen over
  Maven Central: zero new account/GPG-signing setup, authenticates with the
  workflow's own `GITHUB_TOKEN` (no new repo secret). Tradeoff documented in
  `android/README.md`: unlike Maven Central, GitHub Packages requires
  authentication for every read of a Maven artifact, even on a public repo.
- **`GenesisBlockDB.xcframework` → wired into `react-native-genesisdb`'s
  podspec automatically**: `react-native-genesisdb.podspec` gained a
  `prepare_command` that downloads, SHA256-verifies, and unzips the v0.2.0
  release zip during `pod install`, plus `s.vendored_frameworks` pointing at
  the result. **Correction to prior docs**: this never needed a CocoaPods
  Trunk publish — RN autolinking (`use_native_modules!`) picks the podspec up
  directly from `node_modules`, the same mechanism virtually every
  third-party RN native module already relies on for its iOS half. A true
  standalone-SPM-outside-RN registry path for `ios/genesisdb` (which would
  need its own root-level repo, since `.package(url:)` requires
  `Package.swift` at the repo root) is deliberately deferred — smaller
  audience than the RN+CocoaPods path this PR unblocks.
- **`react-native-genesisdb` → npm**: `.github/workflows/release.yml` gained
  an `rn-npm-publish` job publishing the package (unscoped, `--tag beta`) on
  every `v*` tag push, reusing the existing `NPM_TOKEN` secret.
- **Found + fixed while touching `release.yml`**: the `x86_64-apple-darwin`
  build leg was pinned to `macos-13`, and the v0.2.0 tag's run had been stuck
  `queued` with no runner assigned for 9+ hours — GitHub has been winding
  down `macos-13` hosted-runner capacity. Switched to `macos-14` (still
  cross-compiles `x86_64-apple-darwin` fine, same approach the iOS
  device/simulator jobs already use in `mobile-build.yml`).
- **None of this has actually published anything yet** — all three jobs are
  infrastructure gated on the next `v*` tag push; `implementation
  "dev.genesisblock:genesisdb-android:0.1.0"` and `npm install
  react-native-genesisdb` still don't resolve until that happens. See
  `docs/SPEC--MOBILE-SDK.md`'s Phase B DoD checklist and issue #125 for what
  remains (Maven Central, a real SPM registry path, on-device acceptance).

### Added — v0.2.0 GitHub Release: mobile SDK binary assets published

- **`GenesisBlockDB.xcframework.zip` and `genesisdb-android-0.1.0.aar` are
  now real, downloadable release assets** on the
  [v0.2.0 GitHub Release](https://github.com/Freshair129/GenesisBlock/releases/tag/v0.2.0)
  (tagged from main; also the engine's first non-beta release — no `v0.2.0`
  tag/release existed before this despite the engine being at `0.2.0` for
  some time). Both were built by CI (`.github/workflows/mobile-build.yml`'s
  `ios-xcframework`/`android-aar` jobs) from the same main commit, then
  downloaded, packaged, and re-downloaded to independently verify their
  checksums end-to-end before upload:
  - `GenesisBlockDB.xcframework.zip` — device + simulator slices, SHA256
    `8359846a8e668770816e0d84940aead0a85812f5aa67f91e7c2ff8308d37bc72`.
  - `genesisdb-android-0.1.0.aar` — SHA256
    `7c3733065c2fe936d50b5e69e50a4bd958a851f322d48676dc7a9700f54bed77`.
  - **What this unblocks:** `s.vendored_frameworks`/a Gradle `flatDir`
    reference now has a concrete URL + checksum to point at, instead of
    nothing.
  - **What this deliberately does NOT unblock** (documented in
    `ios/README.md`, `android/README.md`,
    `react-native-genesisdb/react-native-genesisdb.podspec`, and
    `react-native-genesisdb/README.md`): no Maven Central/GitHub Packages
    entry for the `.aar`, no CocoaPods Trunk/SPM registry entry for the
    xcframework, no npm publish of `react-native-genesisdb`, no
    `prepare_command` wiring the xcframework into the podspec, and
    `ios/genesisdb/Package.swift` deliberately still links a local host-arch
    build rather than the published binary target — swapping to
    `.binaryTarget(url:, checksum:)` would drop `GenesisDBTests`' ability to
    actually execute, since the xcframework's slices are cross-compiled for
    `aarch64-apple-ios`/`-sim` and cannot run on the build host.
- **Fixed a registration gap in `modules.json`**: `genesisdb-ios` (the
  iOS SwiftPM package shipped in Phase B-1, PR #122) had no surface entry at
  all — only `genesisdb-android` and `react-native-genesisdb` were listed.
  Added it (`path: ios/genesisdb`, `version: 0.1.0`,
  `targetsSchemaVersion: 3`, `minEngineVersion: 0.2.0`), matching the
  Android entry's shape.

### Added — `react-native-genesisdb`'s iOS module wired to the B-1 Swift SDK

- **`GenesisDbModule.swift` is real code, no longer a stub.** Every method
  (open/close/addNode/search/executeHql/retrieveContext/flushIndex) now calls
  the `GenesisDB` actor from `ios/genesisdb`, mirroring
  `GenesisDbModule.kt`'s structure exactly: a small `InstanceRegistry` — its
  own tiny `actor`, the Swift-native equivalent of Kotlin's
  `ConcurrentHashMap` + `AtomicInteger` — maps an opaque `dbId` int to the
  live actor instance (handles never cross the RN bridge as raw pointers,
  same precision-loss rationale as the Android side). No changes needed to
  the existing `@objc`/`.m` bridge signatures or the TS layer — the RN-facing
  contract was already correct, only the Swift method bodies were stubs.
- **Two things this wiring genuinely cannot solve alone** (documented in
  `react-native-genesisdb/README.md` "iOS integration status" and the
  podspec, mirroring `android/build.gradle`'s identical "references an
  unpublished artifact" shape for `genesisdb-android`): the
  `GenesisBlockDB.xcframework` isn't published as a release asset yet, and
  CocoaPods has no mechanism to depend on a Swift Package — a consuming app
  must add `ios/genesisdb` as an Xcode-level SPM dependency alongside
  `pod install`. Neither is verified by this monorepo's CI (no real RN host
  app exists here to resolve either dependency against) — the same
  host-only carve-out already established for the rest of the mobile SDK.

### Added — MARK XVI Phase B-1: iOS Swift SDK

- **`ios/genesisdb/`**: a real SwiftPM package wrapping the C ABI (`src/ffi.rs`)
  — full parity with the Android SDK (B-2), not the 4-method spec snippet:
  `GenesisDB` is an `actor` covering all 16 `genesisdb_*` symbols (add_node,
  search, execute_hql, execute_query_ir, query_ir_capabilities,
  retrieve_context, the 6 relational-surface calls, commit_transaction,
  flush_index), and `Types.swift` mirrors `Types.kt`'s `CodingKeys`/wire
  contract exactly (the C ABI serializes the same un-renamed `serde_json`
  structs as the REST server and the Android JNI bridge — **snake_case**, not
  the napi-rs camelCase in `index.d.ts`).
- Modeled as a Swift `actor` rather than a plain class: every call is
  serialized onto the actor's executor, so — unlike `GenesisDB.kt`, which
  documents "not thread-safe to call concurrently" as a caller obligation —
  a caller here cannot race `close()` against another method by construction.
- **Fixed a cross-platform bug found while porting**: the Rust `ContextPackage`
  struct's `coverage: CoverageReport` field (no `#[serde(default)]`, always on
  the wire) was missing entirely from the Android SDK's `Types.kt`, so every
  Android caller of `retrieveContext` silently lost that data to
  `ignoreUnknownKeys`. Added `CoverageReport` + the field to `Types.kt` and
  its `WireFormatTest.kt` fixture, alongside the Swift version that has it
  from the start.
- **`WireFormatTests.swift`**: pure-Swift, zero-C-dependency tests proving the
  wire contract (mirrors `WireFormatTest.kt` test-for-test) — the same
  no-native-lib property `android-jvm-tests` has.
- **`RoundTripTests.swift`**: a REAL, executed `addNode`/`retrieveContext`/
  `search` round trip against a host-architecture build of the compiled
  engine (not a cross-compiled iOS slice, which can't execute on the build
  machine) — stronger verification than B-2 has today, since Android has no
  accessible on-host JNI execution path in this CI.
- New CI jobs in `mobile-build.yml`: **`ios-xcframework`** assembles the real
  `GenesisBlockDB.xcframework` from `ios-build`'s staticlib output via
  `xcodebuild -create-xcframework` (mirrors `android-aar`); **`ios-swift-tests`**
  runs both Swift test targets. `ios/**` added to the workflow's path triggers.
- `Package.swift` links directly against a locally-built
  `libgenesis_block_native.a` via `unsafeFlags` — explicitly documented as the
  CI/local-dev shape, not the eventual published package (which swaps to a
  `.binaryTarget(url:, checksum:)` pointing at a release xcframework asset,
  same as the spec's original plan).
- Not yet done (same host-only carve-out already established for B-2/B-3):
  publishing the xcframework as a release asset, on-device/Xcode-project
  acceptance, and wiring `react-native-genesisdb`'s iOS stub to this package.

### Fixed — four storage-readiness-audit items (security + ops)

- **Collection-name path traversal closed.** A collection name becomes part of
  six on-disk filenames (`vec_`, `meta_`, `fvec_`, `bqmean_`, `sq8scale_`)
  joined to the DB directory and was never validated. Worse than the audit
  recorded: besides `create_collection`, the CRDT/WAL `replay_vector` path
  auto-provisions a collection from a **peer-supplied** name. Names are now an
  ASCII `[A-Za-z0-9_-]` allowlist, ≤64 chars, no leading `-`. The caller path
  fails loudly; the remote path drops the event as inert (one malformed peer
  event must not abort recovery). Tests pin the *filesystem* outcome — a
  sentinel directory beside the DB stays empty — not just the error string.
  Loading an existing manifest is deliberately not re-validated.
- **HQL fuzz suite was vacuous.** `must_not_panic` called `catch_unwind` and
  discarded the result, so a genuinely panicking parser still passed every
  "must not panic" case. The result is now asserted; the whole suite still
  passes, which is the first time that has actually been demonstrated.
- **REST server graceful shutdown + final checkpoint.** `genesis-db-server`
  ran `axum::serve` with no shutdown handling: Ctrl-C/SIGTERM killed the
  process mid-flight and `Drop` never ran. It now drains in-flight requests on
  SIGINT (and SIGTERM on Unix — what every container runtime sends), then
  `save_state()`s on the quiescent engine. Durability never depended on this
  (the journal holds every acked write); the checkpoint buys an instant next
  start and, under `frontier_only`, the fold that bounds journal growth. A
  failed final checkpoint is logged, never hidden, never fatal.
- **Backup restore no longer demands exact `engine_version` equality**, which
  made every bundle unrestorable after any release (0.2.0 → 0.2.1 refused even
  with an identical on-disk schema). Compatibility is governed by the existing
  `schema_version` gate — newer schema still rejected, older still migrated on
  open. New test restores a same-schema bundle stamped with an older engine
  version and checks the graph inside it.

### Added — WP-3.3 moat follow-ups measured (libSQL/DiskANN + real bge-m3 corpus)

- **`docs/REPORT--MOAT-FOLLOWUPS.md`**: both follow-ups the WP-3.3 decision made
  prerequisites for public moat positioning are now measured, and **neither
  caveat was hiding a weakness**.
- **Real embeddings are conservative, not flattering.** At matched N (11,266 ×
  1024), real bge-m3 vectors *raise* every vector-touching ratio versus the DIY
  SQLite assembly — q1 52.3× → 67.2×, q3 28.7× → 36.9×, q4 16.2× → 22.8× — while
  the graph-only control moves −4% (noise) and the baseline's O(N) scan barely
  moves. Clustered vectors navigate the HNSW graph in fewer hops; a brute scan
  is distribution-blind.
- **libSQL 0.9 + native DiskANN does not close the gap.** It beat the brute scan
  by only 1.21× (synthetic) / 1.88× (real) and left the engine ~12–13× ahead on
  the vector axis and ~47× on the fused shape, at **8.5×–11.8× the engine's
  ingest cost**. The decision doc's prediction held for the fused shapes and
  understated the single-axis result.
- **New `moat-libsql` binary** (`benches/moat_libsql.rs`, gated behind the
  `libsql-baseline` feature, kept out of `bins`). It is a separate process by
  necessity: `libsql-ffi` and `rusqlite` both export the bundled `sqlite3_*`
  symbols, so one binary either fails to link (LNK2005) or silently resolves
  every call into one implementation — which would have run the engine's own
  `projection.sqlite` and the competitor on the same accidental SQLite.
  Comparability is preserved by identical seed/corpus/protocol/host instead, and
  the deviation is disclosed in the report.
- **New `benchmark/gen_corpus_bge_m3.py`**: builds a real-embedding corpus
  deterministically from this repository's own prose via a local Ollama bge-m3,
  L2-normalized, with a provenance manifest (model, dim, count, sha256, source
  commit, extraction rules) that the bench copies into `result.json`. Resumable:
  embeddings append row-by-row so an interrupted run continues.
- moat-bench gains `GB_MOAT_VECTORS` (real-corpus mode); both runs pass
  `verify_report.py`. `DECISION--WP33-GNSE-BACKLOG` and `REPORT--G3-MOAT-VERDICT`
  updated with the outcome — public positioning is unblocked, with the standing
  rule that no ratio is quoted without its N.

### Added — E2 vector time-travel (epoch stamps + filtered ANN + horizon-aware compaction)

- **`tx_as_of` now works on vector SEARCH with epoch-complete candidates**
  (SPEC--GENESISDB-EPOCH-HNSW §3.1/§3.3): `NodeMetadata` gains
  `created_seq`/`retired_seq` stamps (meta snapshot **v2, `GBP2` magic,
  manifest `mv: 2`** — GBP1/bincode/V0 snapshots migrate on load with zeroed
  stamps, "always existed"), and `hybrid_search` under a tx selector
  enumerates candidates by the epoch predicate instead of gating on the live
  map: retracted nodes resurrect, not-yet-committed nodes drop, and a
  re-embedded node resolves to the embedding that was current at t (the
  displaced row is stamped at re-embed time, making the dedupe epoch-correct).
- Candidate generation: `hnsw_rs::search_filter` (first use — the plain
  `search()` was already a `filter: None` wrapper) with an **exact-arena-scan
  fallback** when the predicate's survivor fraction falls below 10% or the
  collection is small — the standard filtered-ANN failure handled correct-by-
  construction; historical queries are audit-shaped. Resolution goes through
  the `node_versions` chain at t (`tx_view_node`, shared with the E1 graph
  path); the old post-resolution `apply_tx_view` pass is removed.
- **Every staging path is persist-first** so the frame's own seq stamps
  `created_seq` (add_node/add_vector reordered; consensus/sync Vector and
  Node arms reordered — extending the Slice-0 rationale); journal replay and
  transactions stamp with the replayed frame's seq.
- **Compaction respects the horizon** (§3.4): a non-live row survives iff
  `retired_seq >= history_horizon()` under a history-retaining profile —
  compaction no longer destroys history the journal still retains. Under
  `frontier_only` the filter reduces to the old live-set behavior exactly
  (C4 cost neutrality); as a side effect, re-embed orphan rows are now
  reclaimed there instead of lingering until the node dies.
- **Fixed (current view)**: a re-embedded node can no longer be ranked by its
  ORPHANED old embedding — pre-E2 the stale HNSW slot could outrank the
  node's current vector until compaction reclaimed it; the current view now
  skips stamped (historical) rows. Pre-epoch migrated rows keep the old
  last-writer-wins behavior.
- Capabilities (C5): `temporal.tx_as_of` upgrades to `"epoch_candidates"`;
  new `temporal.vector_tx_as_of` advertises the implementation + the active
  retention profile.
- moat-bench gains **q6, the vector-time-travel row** (1000-node tx cohort in
  its own collection vs a stamped brute-scan SQLite table; capability row,
  excluded from `min_cross`).
- New `tests/epoch_e2_tests.rs` (quadrants, historical-embedding re-embed,
  snapshot reopen + journal replay, compact-then-query under full vs
  frontier_only, GBP1→GBP2 migration, SEARCH/TRAVERSE agreement);
  `meta_format_migration_tests` updated to the GBP2 container.

### Added — E1 retired-adjacency overlay (epoch-complete tx_as_of traverse)

- **`tx_as_of` can now resurrect retracted nodes on TRAVERSE** (SPEC--GENESISDB-
  EPOCH-HNSW §3.2, phase E1): `retract_node` moves incident edges into a
  retired-adjacency overlay (`edges_retired` + string-keyed adjacency, stamped
  with the NodeRetract frame seq) instead of destroying them; a new
  `neighbors_tx_view` BFS unions the overlay and resolves every candidate —
  including nodes absent from current indexes — through the `node_versions`
  chain at the selector. The formerly `#[ignore]`d WP-3.1 RED test
  (`matrix_retraction_belief_before_still_serves`) is un-ignored and green.
- Overlay lifecycle: persisted in `edges_retired.bin` (snapshot instant-load),
  rebuilt by journal replay and CRDT reconcile (retraction paths are now
  persist-first everywhere, extending the Slice-0 rationale), and **cleared on
  every successful fold** — the fold stays the single history-destruction
  boundary (I6), so `frontier_only` keeps its exact cost profile.
- Capabilities: additive `temporal.tx_as_of_traverse = "epoch_candidates"`;
  the existing `tx_as_of` key is unchanged (SEARCH stays post-resolution
  until E2 vector epoch candidates).
- New `tests/epoch_e1_tests.rs`: multi-hop through a retracted intermediate,
  snapshot-reopen + journal-replay overlay rebuild, fold-clears + loud
  `beyond_horizon`, retract-then-recreate view separation.

### Added — epoch-HNSW spec (draft)

- **`docs/SPEC--GENESISDB-EPOCH-HNSW.md`**: design spec for the WP-3.3-funded
  epoch-segmented indexes / vector time-travel line. Three mechanisms, no journal
  format change: epoch stamps on vector metadata (`meta` snapshot v2, migrate
  ladder), a retired-adjacency overlay that turns the WP-3.1 RED test green
  (tx_as_of resurrection of retracted nodes), and filtered-ANN + exact-scan-floor
  vector time-travel with horizon-aware compaction. Phased E1/E2 with per-phase
  DoD; true per-epoch HNSW sub-indexes stay evidence-gated (E3).

### Decided — WP-3.3 GNSE backlog gate (USER)

- **Fund selectively** (`docs/DECISION--WP33-GNSE-BACKLOG.md`): epoch-segmented
  HNSW / vector time-travel is FUNDED off the WP-3.2 PROCEED evidence (it also
  un-ignores the WP-3.1 RED test — `tx_as_of` resurrection of retracted nodes);
  segment stores + page cache, CommitFrame prev-hash chain, and SQLite property
  demotion stay deferred on their original §8 triggers. Scheduled follow-ups
  before any public moat positioning: libSQL DiskANN baseline row + real-corpus
  (bge-m3) moat run. This closes the GNSE remediation line (WP-0.1 → WP-3.3).

### Added — WP-3.2 G3 moat bench + PROCEED verdict

- **`moat-bench`** (`benches/moat_bench.rs`, bins-gated `[[bin]]`;
  wrappers `benchmark/run_moat_bench.{sh,ps1}`): the engine's fused
  vector+graph+AS-OF jobs vs the DIY single-SQLite-file assembly ROUND2
  named as the primary embedded competitor (brute f32 scan =
  sqlite-vec-stable model, recursive-CTE hops, shared Rust RRF glue,
  audit-history temporal pattern). Both sides in-process in one Rust
  binary — reported wins are lower bounds. Deterministic seeded corpus,
  clone-and-run, trust-gated through `verify_report.py`.
- **Verdict: PROCEED** (`docs/REPORT--G3-MOAT-VERDICT.md`, consumed by the
  WP-3.3 decision gate). At 100k×1024: Q1 fused 187.9×, Q3 114.9×,
  controls 92×/83.9× — every cross-dimension query clears the ROUND2
  G3-e ≥5× bar by an order of magnitude, and the advantage grows with
  dimension span (Q1 > Q3 > controls). The baseline structurally fails
  2/5 WP-3.1 bitemporal correctness scenarios (no tx axis; no provenance
  identity). Disclosed honestly: ingest stays the engine's weak side
  (141.9 s vs 33.1 s bulk), Q2 skipped until the FTS axis (S3) ships,
  synthetic corpus carries no recall claim.

### Added — WP-3.1 bitemporal correctness suite

`tests/bitemporal_matrix_wp31_tests.rs` — the correctness matrix the DIY
SQLite assembly must also pass in the moat bench (interview ROUND2 G3-e bar;
GNSE plan Phase 3 "Prove or kill"). Tests only, no engine changes:

- **valid×tx four-quadrant matrix** on a superseded node — including the
  two-axis case: "at commit S1 the recorded belief about 2022 had an OPEN
  validity window" vs today's closed one.
- **Retraction across tx time** — current view and at-or-after beliefs drop
  the node. The belief-BEFORE half is a deliberate `#[ignore]`d TDD RED
  test: the disclosed `implemented_post_resolution` semantics (WP-2.2)
  cannot resurrect a retracted node from current indexes; un-ignore when
  epoch-segmented indexes land (WP-3.3 gate). Not rewritten to assert the
  gap as expected behavior.
- **Correction-after-the-fact** — a retroactive `retract_edge` changes the
  answer to the same valid-time question across tx time.
- **Interval-overlap boundaries** — `valid_from <= as_of < valid_to`
  (start inclusive, end exclusive) probed at all four boundary points.
- **Audit reconstruction** — create → 2 supersessions → retract fully
  reconstructed from the `node_versions` chain; the WP-2.3 `caused_by`
  auto-chain walked backwards v3→v2→v1 purely from stored identities.
- **Reopen survival** — chain length and both temporal axes re-verified
  after a process restart.

### Added — WP-2.3 caused_by auto-chain + queryable recorded_at

- **`caused_by` auto-chain on supersede** (`supersede_node`): when the caller
  passes no provenance, the new version's `caused_by` now defaults to the
  identity of the version the supersession closed — `<id>@<frame_seq>` of
  the closing frame — instead of staying empty. The embedded frame seq
  resolves that exact version back through the WP-2.1 `node_versions`
  tx-time chain, so every unannotated supersession is self-documenting.
  An explicit caller-provided `caused_by` always wins, unchanged.
- **`recorded_at` queryable in HQL pattern clauses**: `qual_tail` (grammar
  `src/query/hql.pest`, kept in sync with `src/query/ast.rs`) gains a
  `recorded_at` accessor — `e.recorded_at` works in WHERE, ORDER BY, and
  RETURN of `MATCH` patterns. Edge bindings project their tx-time ingestion
  timestamp (RFC3339, so string comparison is chronological); node bindings
  resolve to null (NodeOutput carries no `recorded_at`), mirroring the
  `score`/`depth` convention. First time tx-time is reachable from query
  text rather than only via the `node_versions` API.

New suite: `tests/wp23_semantics_tests.rs` (auto-chain resolves through the
version chain, explicit provenance wins, RETURN projection, WHERE filtering
incl. the null-on-node case).

### Added — WP-2.2 tx_as_of + the as_of semantics fix

- **`temporal.tx_as_of` on the Typed Query IR** (search + traverse; NAPI,
  REST, and new FFI/JNI surfaces `genesisdb_execute_query_ir` /
  `genesisdb_query_ir_capabilities` + JNI mirrors; `include/genesisdb.h`
  regenerated): a replica-local commit-seq selector — "what did this replica
  believe at its commit N". Selectors below `history_horizon()` fail
  explicitly with `beyond_horizon` (ADR D4 rule 2). Interim semantics,
  disclosed by capabilities as `tx_as_of: "implemented_post_resolution"`:
  candidates come from current indexes, then each result node is re-resolved
  through the WP-2.1 version chain at N — nodes with no committed version
  at-or-below N, or retracted at N, are dropped (epoch-segmented indexes
  remain gated GNSE backlog).
- **`as_of` (valid-time) semantics fix** in `hybrid_search` and `neighbors`:
  a node whose current version postdates the selector now resolves its
  historically valid version from the chain (with its closed validity
  window) instead of being silently hidden — closing the superseded-node
  defect the GNSE review flagged, which `temporal_queries_tests` had
  codified as expected behavior (that assertion is now inverted). Chain
  lookup runs only when the current version fails the window (cold path);
  below the fold horizon the node stays hidden, matching the disclosed
  retention forfeit.

New suite: `tests/tx_as_of_wp22_tests.rs` (historical resolution,
not-yet-committed drop, beyond-horizon rejection, superseded-version
resolution on the search path).

### Added — WP-2.1 node_versions (tx-time version chain)

The first queryable tx-time surface (GNSE plan Phase 2): a per-entity version
chain in the SQLite projection, keyed by the LOCAL frame seq (ADR D2 —
replica-local commit order; PROJECTION_SCHEMA_VERSION 2 → 3, additive
migration).

- Every committed `Node` frame appends a chain row (deliberately NOT
  clock-LWW-gated — the chain records what was committed, in frame order);
  `NodeRetract` frames append retraction markers, so resolve-at-commit past a
  retraction answers "retracted", not the last live version. Supersede
  naturally yields close+new row pairs.
- Read API `node_versions(id, at_seq?)` on Storage, NAPI (`nodeVersions`),
  and REST (`GET /v1/node/versions?id=..&at_seq=..`): frame-ordered chain +
  optional resolve-at-commit. Lookup is by id string, so a retracted node's
  chain stays addressable after its interning entry is gone.
- **ADR D4 enforced:** rows below `history_horizon()` are never served (a
  projection rebuild would not recover them — the chain stays strictly
  rebuildable, proven by test), and `at_seq` below the horizon fails
  explicitly with `beyond_horizon` — never silently the current state. Under
  `frontier_only` this means the chain collapses at every fold, exactly the
  forfeit the capabilities surface discloses; under `full`/`budget` (WP-1.3)
  real history accumulates.

New suite: `tests/node_versions_wp21_tests.rs` (chain shape across
supersede, resolve-at-commit, retraction resolution, journal rebuild
identity, beyond-horizon behavior after a fold).

### Added — WP-1.3 retention profiles (ADR D3)

Journal retention is now a per-database setting: `OpenOptions.retention`
(`"frontier_only"` | `"full"` | `"budget:<bytes>"`; REST server env
`GENESIS_RETENTION`; unrecognized values fail `open` loudly — no
silent-default trap).

- **`frontier_only`** (default, unchanged behavior): fold at every
  checkpoint; forfeits tx-time history — and the capabilities surface now
  says so.
- **`full`**: checkpoints never fold; history accumulates as sealed segments
  and the journal retains full post-adoption history. Explicit `compact()`
  still folds.
- **`budget:<bytes>`**: checkpoints fold only when sealed history exceeds the
  byte budget — the bounded-disk contract, retaining up to the budget of
  tx-time history between folds. The active-file seal threshold derives from
  the budget (N/4, clamped to [64 KiB, 64 MiB]) so small (mobile-sized)
  budgets actually seal and trip. Interim semantics: a tripped budget folds
  the whole history window (the ADR's oldest-first partial fold needs a
  state-as-of-boundary materializer — deferred), which still bounds disk.
- **Horizon honesty (ADR I6, previously unreachable from any surface):**
  `query_ir_capabilities` gains `temporal.{history_horizon, tx_epoch_start,
  retention_profile, tx_time_retention}`; `GET /v1/frontier` gains
  `history_horizon` + `retention_profile` (additive); new NAPI
  `historyHorizon()`.
- Tombstone GC (Slice 1) now also runs on non-folding checkpoints, so the
  registry/snapshot stay bounded under `full`/`budget`.
- Deferred, per plan: default flip to `budget:4GiB` (belongs with the WP-2.x
  tx-time landing), the peer-aware retention floor (the sync commit-seq
  cursor is not yet used by requesters), and the archive hook.

New suite: `tests/retention_wp13_tests.rs` (fail-loud parsing, per-profile
fold behavior, bounded-disk under churn with journal-only recovery, horizon/
retention disclosure).

### Added — Slice-1 tombstone retention

Closes the two documented Slice-0 residuals (deletion convergence and the
fold's destruction of retraction history), within an interim 30-day retention
window (`TOMBSTONE_RETENTION_SECS`; policy moves to WP-1.3 retention profiles):

- **Node tombstone registry** (`Storage.tombstones`): every retraction records
  `{clock, retracted_at}`, persisted in `state.json` and re-emitted into the
  fold payload as `NodeRetract` frames — so the deletion survives snapshots,
  folds, and journal-only recovery.
- **CRDT deletion convergence:** `reconcile_state` now gates `Node` upserts by
  tombstone LWW — a stale peer re-push can no longer resurrect a retracted
  node after the origin folds (previously guaranteed resurrection: no local
  copy remained to win LWW). Remote `NodeRetract` events are recorded even
  when no local node is resident, with clock-idempotent re-offer handling.
  A genuinely newer upsert clears the tombstone (legitimate re-create).
- **Retracted edges survive the fold** within the retention window, restoring
  `retract_edge`'s documented time-travel contract (`as_of` before the
  retraction / `include_invalid`) after a checkpoint when the journal is the
  only surviving copy. Retention comparisons parse RFC3339 (no lexicographic
  string compare); unparseable stamps are conservatively retained.
- **GC at the fold boundary:** tombstones and retracted edges older than the
  window leave the fold payload; expired tombstones are dropped from the
  registry and the snapshot. Known residual: a peer partitioned longer than
  the window can still resurrect a delete — WP-1.3 territory.
- `Event::NodeRetract` gains a `retracted_at` field (`serde(default)`;
  additive within the unreleased v3 format — no schema bump).

New regression suite: `tests/durability_slice1_tests.rs` (stale-push LWW,
fold/snapshot/journal-only tombstone survival, legitimate re-create,
retracted-edge time travel after fold, GC at the window).

### Fixed — Slice-0 durability (SCHEMA_VERSION 2 → 3)

Four acked-write-loss / resurrection defects from the 2026-08-19 storage-
readiness audit (RCA--SLICE0-DURABILITY). All four were silent — no test
injected I/O errors or crashed inside the checkpoint window.

- **Journal write errors are no longer swallowed.** The WAL writer thread
  previously discarded `write_all`/`flush` results and acked on `sync_all()`
  alone, so an ENOSPC/EIO frame was acknowledged as durable. The ack now
  requires the whole batch's write + flush + fsync to succeed, and after any
  I/O failure the writer is poisoned (every append refused with a failed ack)
  until a successful fold rebuilds a clean active file — a torn tail can no
  longer sit under later "successful" appends that replay would never reach.
- **`retract_node` is journaled.** Node retraction (including the hourly
  autonomic TTL/orphan prune) used to mutate memory only; a crash before the
  next checkpoint resurrected the node and its cascaded edges on replay, and
  CRDT peers re-pushed it. A new `Event::NodeRetract` frame is now persisted
  *before* the in-memory removal, replayed as a removal, applied to the SQLite
  projection (props/labels rows deleted, including on rebuild), replicated via
  `events_since` (clock-stamped), and applied with node-style LWW on
  `reconcile_state`. **This new frame kind is why SCHEMA_VERSION bumps to 3:**
  older engines silently skip unknown journal events — a downgrade would
  silently resurrect deletions — so it fails closed instead.
- **A checkpoint can no longer write a snapshot without its journal cursor.**
  If `build_compacted_wal()` failed, `save_state` used to write `state.json`
  with no `journal` cursor; the next open then skipped replay entirely,
  silently dropping every write acked after the save. `save_state` now aborts
  loudly on a failed payload build (the previous snapshot + full journal
  remain a complete recovery source), propagates `state.json` write errors,
  and the cursor-less recovery branch — still reachable for pre-frontier
  snapshots — now replays the full journal on top of the instant load
  (idempotent LWW; same one-time duplicate-arena-rows tradeoff as the legacy
  `wal_frontier` branch).
- **A stale snapshot older than a completed fold is no longer trusted.** In
  the crash window between `journal_fold` and the `state.json` rename, the old
  snapshot still holds state that was deleted and folded away; base-segment
  replay can only add, so recovery resurrected it. Open now detects
  `history_horizon() > snapshot frontier` and recovers from the journal alone
  (the base segment is a complete recovery source, invariant I8).

New regression suite: `tests/durability_slice0_tests.rs` (crash-image and
clean-reopen retraction survival, stale-snapshot-vs-fold, cursor-less
snapshot tail recovery).

### Changed — BREAKING (on-disk + API), WP-1.2 framed journal
- **On-disk journal format (SCHEMA_VERSION 1 → 2).** `genesis-graph.wal` (JSONL)
  is replaced by a framed journal: `wal/active.gwal` (GWA1 header + frames
  `[u32 len | u64 commit_seq | u32 crc32c | SignedEvent JSON]`) plus sealed,
  zstd-compressed `journal/*.gseg` segments. Frames wrap the **original**
  event bytes, so peer signatures now survive checkpointing (previously
  compaction re-signed every event with the local key). Migration is automatic
  and **one-way**: on first open the legacy WAL is sealed as segment 0
  (`kind=legacy_jsonl`, recovery-only). An engine older than the on-disk
  version now fails closed with `SCHEMA_VERSION_UNSUPPORTED` — never a partial
  read. See [ADR--GENESISDB-JOURNAL-HISTORY](docs/adr/ADR--GENESISDB-JOURNAL-HISTORY.md)
  and [SPEC--GENESISDB-JOURNAL-FORMAT-V1](docs/SPEC--GENESISDB-JOURNAL-FORMAT-V1.md).
- **Checkpoint folds instead of truncating.** `save_state()` now folds the
  journal into a base segment (live state) rather than rewriting the WAL file,
  so the journal remains a complete standalone recovery source at every instant
  and the seal is durable *before* the snapshot manifest advances (invariants
  I7/I9). Disk stays bounded exactly as before (interim `frontier_only`
  retention profile; budget profiles land in WP-1.3).
- **`stableFrontier()` / `GET /v1/frontier` semantics changed.**
  `stable_frontier` is now the **frame** frontier — the commit sequence of the
  last durable journal frame, advancing on *every* mutation. The previous
  meaning (sequence of the last transaction) is now `txnFrontier()`, and that
  is the value `GenesisTransaction.expected_frontier` must be CAS'd against.
  `GET /v1/frontier` returns `{"frame": N, "txn": M}` instead of a bare number;
  SDKs reading the old scalar should read `.txn`. `CommitResult.commit_sequence`
  is now the transaction's frame stamp.
- **Transaction sequences are replica-local.** `GenesisTransactionEvent.commit_sequence`
  is demoted to `origin_commit_seq` (serde alias keeps old WAL lines parsing);
  a replica no longer merges a peer's sequence into its own counter, which also
  removes a `applied_transactions` uniqueness collision that could abort a whole
  reconcile batch.

### Added
- **HQL Cypher-style graph patterns (path 1):** a fifth HQL command,
  `MATCH (a:Label {k:v})-[r:REL]->(b) ...`, matching linear path patterns by
  deterministic left-to-right expansion over the graph indices — **no query
  planner**. Supports node label/prop constraints, edge type + direction
  (`->`/`<-`/`-`), `{id:"…"}` anchoring, and variable-qualified
  `WHERE`/`ORDER BY`/`LIMIT`/`RETURN` (`a`, `a.id`, `a.label`, `a.prop.<key>`)
  plus `AS OF`. `MATCH (` routes to patterns; `MATCH <t> SIMILAR TO …` remains
  the hybrid command (no breaking change). v1 is linear-path-only (no
  variable-length `*`, branching, or `OR`). Lands on both NAPI (`executeHql`)
  and REST (`/v1/query/hql`) with no signature change. See
  `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md`; tests in
  `tests/hql_cypher_tests.rs`.

## [0.2.0] - 2026-06-29

First non-beta release. Lays the MARK XVI foundation for embedding
GenesisBlockDB in-process on mobile (iOS/Android), the same way SQLite ships
inside an app — no server, no network.

### Added
- **Mobile build features (`Cargo.toml`):** `mobile` builds the storage core for
  iOS/Android targets (no napi, no bins, no sysinfo); `ffi` exposes the C ABI
  layer. `sysinfo` (bench-only RSS probe) is now an optional dependency owned by
  the `bins` feature, so it never enters a mobile build.
- **C FFI layer (`src/ffi.rs`):** 8 `#[no_mangle]` C symbols
  (`genesisdb_open`/`close`/`add_node`/`search`/`execute_hql`/
  `retrieve_context`/`flush_index`/`free_string`) over the synchronous `Storage`
  core, gated behind the `ffi` feature. `catch_unwind`-guarded so panics never
  cross the boundary; JSON-in/JSON-out mirrors the REST/NAPI contract. Consumed
  by the future iOS xcframework and Android JNI bridge.
- **Mobile cross-compile CI (`.github/workflows/mobile-build.yml`):** `ios-build`
  (macOS runner → `aarch64-apple-ios` + simulator), `android-build` (Linux +
  cargo-ndk → arm64 + armv7), and `host-mobile-check`
  (`cargo test --no-default-features --features mobile`).
- **Spec & roadmap:** `docs/SPEC--MOBILE-SDK.md` (Phase 0/A/B, Levels A+B) and the
  MARK XVI section in `ROADMAP.md`.

### Notes
- No engine behavior change for existing surfaces. `Storage::open(OpenOptions)`
  already takes a caller-supplied DB path, so mobile sandboxing needs no core
  change. iOS/Android cross-compile is validated only by the new CI on
  GitHub-hosted runners (the dev host is Windows).

## [0.1.0-beta.2] - 2026-06-25

### Added
- CI test gate (`.github/workflows/test.yml`): runs `cargo test`
  (Linux/Windows/macOS, via `--no-default-features`) and `npm test`
  (Linux/Windows/macOS) on every PR and push to `main`. Replaces
  the prior situation where the only `main` workflow was a perf audit that
  skipped itself on Linux, so no CI gate actually exercised the test suites.
- Security audit gate (`.github/workflows/security.yml`): `cargo audit` against
  the RustSec advisory database on push/PR and weekly.
- **Version control (semver SSOT):** `scripts/version.mjs` keeps the engine
  version (`x.y.z[-prerelease]`) in lock-step across `Cargo.toml`,
  `package.json`, and `modules.json` (`npm run version:get|check|set|bump`). CI
  `version-consistency` job fails the build on drift.
- **Update system:**
  - `GET /v1/version` (REST) and `versionSync()` (NAPI) report
    `{engine_name, version, schema_version}` so clients/ops can see the running
    version. Engine version is baked from `CARGO_PKG_VERSION` (`ENGINE_VERSION`).
  - `scripts/check-update.mjs` (`npm run update:check`) — notify-only update
    check against the npm registry (never auto-installs).
  - Schema-version compatibility gate on open: a database written by a newer
    engine is refused with a clear error (forward-incompat protection); older /
    pre-versioned snapshots open via the existing migration path.
- `SECURITY.md` (vulnerability reporting policy), `docs/OPERATIONS.md` runbook,
  and this `CHANGELOG.md`.

### Changed
- **core/napi split (#161):** the napi bindings are now gated behind a
  default-on `napi-bindings` feature. With it off
  (`cargo build/test --no-default-features`), the storage core, REST server, and
  all integration tests compile as plain native binaries with no `napi_*`
  symbols — so they link and run on Linux. The CI test gate now runs `cargo test
  --no-default-features` on **all three** platforms (Linux/Windows/macOS); the
  default build still produces the napi cdylib unchanged. `temporal_queries_tests`
  was converted from the async `GenesisDatabase` wrapper to the sync `Storage`
  core so it runs in both modes.
- `package.json`: native build moved from the `install` script to `prepare`, so
  registry consumers receive the prebuilt platform addon (via napi
  `optionalDependencies`) instead of being forced to compile Rust on every
  `npm install`. Local dev clones still build on install.

### Fixed
- Deterministic rerank: the rerank+compaction path was nondeterministic under
  load (two same-sign BQ vectors collapse to one binary code, so the approximate
  HNSW prefilter could surface only one of the tied pair). When a rerank sidecar
  is present and the over-fetch already covers ~every slot, the full sidecar is
  now scored exactly. Large collections keep the HNSW path (recall benchmarks
  unaffected).
- `/v1/query/hql` now accepts both the raw-JSON-string body and the
  `{"query": "..."}` object body the Python/Go SDKs send (was: raw string only,
  which rejected every SDK request).
- Hardened panic paths in the engine: NaN-safe sort in hybrid search
  (`partial_cmp` no longer `unwrap`s), and the gossip/swarm UDP setup
  (`local_addr` / `set_broadcast`) now fails gracefully instead of panicking the
  background task.

## [0.1.0-beta.1] - 2026-06-25

First beta cut.

### Added
- WAL compaction: `WalMsg::Checkpoint` truncates the WAL through the writer
  thread; compaction is wired into `save_state()`. Bounds previously-unbounded
  WAL growth.
- `Cargo.lock` is now tracked.
- `modules.json` multi-surface version manifest (engine + 6 client surfaces,
  schemaVersion 1).
- napi cross-compile matrix and npm publish on version tag
  (`.github/workflows/release.yml`).

[Unreleased]: https://github.com/Freshair129/GenesisBlock/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Freshair129/GenesisBlock/compare/v0.1.0-beta.2...v0.2.0
[0.1.0-beta.2]: https://github.com/Freshair129/GenesisBlock/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/Freshair129/GenesisBlock/releases/tag/v0.1.0-beta.1
