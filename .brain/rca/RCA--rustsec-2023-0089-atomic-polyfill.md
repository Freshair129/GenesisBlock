# RCA: RUSTSEC-2023-0089 in the Postcard Test Dependency

## Status / Date

Root cause confirmed; fix applied and local validation passed; hosted CI pending / 2026-09-27.

## Symptom

GitHub issue #176 reports `atomic-polyfill 1.0.3` as unmaintained (RUSTSEC-2023-0089). The locked crate is reached from GenesisBlockDB's dependency graph when all target platforms and Cargo features are inspected. The smallest reproduction is:

```text
cargo tree --offline --locked --target all -e features -i atomic-polyfill
```

The command reports `genesis-block-native -> [dev-dependency] postcard default -> postcard heapless-cas -> heapless cas -> atomic-polyfill 1.0.3`.

## Evidence

- `Cargo.lock` pins `atomic-polyfill` 1.0.3 and lists it under `heapless` 0.7.17.
- The production `postcard` declaration in `Cargo.toml` disables default features and enables only `alloc`.
- The `postcard` dev-dependency enables `use-std` but leaves default features enabled.
- The locked `postcard` 1.1.3 manifest defines `default = ["heapless-cas"]`; that feature enables `heapless/cas`. The locked `heapless` 0.7.17 manifest defines `cas = ["atomic-polyfill"]` and has target-specific `atomic-polyfill` dependencies.
- `cargo tree --offline --locked --target all -e features -i atomic-polyfill` reproduces the complete reverse feature path above.
- Existing metadata migration tests use `postcard::to_allocvec` and `postcard::from_bytes`; they do not use the heapless serialization API.
- RustSec classifies this as an unmaintained-package advisory, not a vulnerability, and lists `portable-atomic` as a possible alternative. No replacement is needed if the unused feature path is removed.

## Root Cause

The test-only `postcard` dependency declaration omitted `default-features = false`. That enabled Postcard's default `heapless-cas` feature for test targets, which enabled `heapless`'s `cas` feature and brought the unmaintained `atomic-polyfill` crate into the all-target lock graph. The production declaration already disables those defaults; GenesisBlockDB's serialization code uses the alloc-backed APIs, not heapless.

## Why the Issue Escaped Detection

The Postcard migration tests verify snapshot round-tripping and legacy reads, but do not inspect Cargo's resolved feature graph. The production dependency declaration had the correct feature restriction while the separate dev-dependency did not. On the default host target, `atomic-polyfill` is target-specific and the ordinary inverse dependency query prints no path; the finding becomes visible with `--target all`.

## Fix (Decided)

Set `default-features = false` on the `postcard` dev-dependency and retain `use-std`. Postcard's `use-std` feature includes `alloc`, which supplies the APIs used by the existing tests. This removes the unused heapless atomic-polyfill path without changing production behavior or adding a new dependency.

## Proposed Prevention

Keep default features explicitly disabled on every Postcard dependency declaration unless a call site requires them. Include an all-target Cargo feature-tree check when auditing locked RustSec advisories, so target-specific optional dependencies are not missed.

## Outcome (Measured)

- `cargo tree --offline --locked --target all -e features -i postcard` shows only the `alloc` and `use-std` Postcard features; `heapless-cas` is absent.
- The all-target lockfile resolution removed `atomic-polyfill`, `critical-section`, `heapless`, and their now-unused transitive packages from `Cargo.lock`. The inverse Cargo tree query for `atomic-polyfill` now reports that the package ID matches no packages.
- `cargo test --offline --no-default-features --test meta_format_migration_tests` passed on Windows: 3 passed, 0 failed.
- `git diff --check` passed. Local `cargo audit` was not available (`no such command: audit`); the hosted Security Audit workflow remains the RustSec audit verification.
