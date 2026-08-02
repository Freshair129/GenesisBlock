---
status: current
---

# Deploy Checklist: GenesisBlockDB v0.2.0 (First Public Release)

**Date:** 2026-06-29 | **Version:** 0.2.0 | **Registry:** npm (`@freshair129` scope)

---

## Pre-Release

- [x] CI green on main (Tests + Perf Audit + Mobile — all passed PR #41)
- [x] CHANGELOG.md updated for 0.2.0
- [x] SECURITY.md present
- [x] LICENSE present
- [ ] Version consistency: `Cargo.toml` = `package.json` = CHANGELOG heading (both show 0.2.0, verify with `npm run agents:validate`)
- [ ] **Soak test passes 12h** — running on C: SSD, auto-stops if disk < 2 GB
- [ ] No `TODO`/`FIXME` blockers in hot paths (`src/lib.rs`, `src/router.rs`)
- [ ] `cargo clippy --no-default-features` clean
- [ ] `cargo test --no-default-features` — all integration tests pass locally
- [ ] `npm test` — NAPI + MCP surface tests pass

## npm Publish Prerequisites

- [ ] **NPM_TOKEN** repo secret configured (Settings → Secrets → Actions)
- [ ] `package.json` does NOT have `"install": "npm run build"` — consumers must get prebuilt addon, not recompile Rust
- [ ] `napi` `optionalDependencies` wired for all 4 targets:
  - `x86_64-unknown-linux-gnu`
  - `x86_64-pc-windows-msvc`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- [ ] `prepublishOnly` script = `napi prepublish -t npm`
- [ ] Dry-run: `npm publish --dry-run` — correct file list (no `.node` in main pkg, no test fixtures)
- [ ] `.npmignore` or `"files"` excludes: `tests/`, `benches/`, `docs/`, `target/`, `*.rs`

## Tag & Release

- [ ] Create annotated tag: `git tag -a v0.2.0 -m "v0.2.0: first public release"`
- [ ] Push tag: `git push origin v0.2.0`
- [ ] **Wait for `release.yml` workflow** — builds 4 platform `.node` addons → publishes to npm
- [ ] Verify on npmjs.com: package visible, correct version, 4 platform packages present
- [ ] Create GitHub Release from tag with CHANGELOG excerpt

## Post-Release Verification

- [ ] `npx @freshair129/genesis-block-native` — smoke test on clean machine or CI
- [ ] MCP server starts: `npm run mcp:start` (fresh install)
- [ ] REST server starts: `cargo run --no-default-features --features bins --bin genesis-db-server`
- [ ] Basic round-trip: ingest node → search → verify result (npm/MCP/REST)

## Rollback Plan

| Action | Command |
|--------|---------|
| Unpublish npm (within 72h) | `npm unpublish @freshair129/genesis-block-native@0.2.0` |
| Publish hotfix | Bump to 0.2.1, fix, tag, push |
| Delete tag | `git tag -d v0.2.0 && git push origin :refs/tags/v0.2.0` |

## Rollback Triggers

- Platform addon fails to load on any supported OS
- Soak test reveals data corruption or memory leak before tag
- npm publish includes secrets, credentials, or test fixtures
- Critical bug reported within first 24h

## Known Gaps (non-blocking, track for v0.2.1+)

- [ ] Obsidian plugin + MCP integration
- [ ] SBOM generation at release time
- [ ] Embedding-model provenance field in metadata
