# Changelog

All notable changes to GenesisBlockDB are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
