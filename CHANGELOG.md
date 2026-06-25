# Changelog

All notable changes to GenesisBlockDB are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI test gate (`.github/workflows/test.yml`): runs `cargo test` (Windows/macOS)
  and `npm test` (Linux/Windows/macOS) on every PR and push to `main`. Replaces
  the prior situation where the only `main` workflow was a perf audit that
  skipped itself on Linux, so no CI gate actually exercised the test suites.
- Security audit gate (`.github/workflows/security.yml`): `cargo audit` against
  the RustSec advisory database on push/PR and weekly.
- `SECURITY.md` (vulnerability reporting policy) and this `CHANGELOG.md`.

### Changed
- `package.json`: native build moved from the `install` script to `prepare`, so
  registry consumers receive the prebuilt platform addon (via napi
  `optionalDependencies`) instead of being forced to compile Rust on every
  `npm install`. Local dev clones still build on install.

### Fixed
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

[Unreleased]: https://github.com/Freshair129/GenesisBlock/compare/v0.1.0-beta.1...HEAD
[0.1.0-beta.1]: https://github.com/Freshair129/GenesisBlock/releases/tag/v0.1.0-beta.1
