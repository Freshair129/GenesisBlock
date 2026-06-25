# Changelog

All notable changes to GenesisBlockDB are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/Freshair129/GenesisBlock/compare/v0.1.0-beta.2...HEAD
[0.1.0-beta.2]: https://github.com/Freshair129/GenesisBlock/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/Freshair129/GenesisBlock/releases/tag/v0.1.0-beta.1
