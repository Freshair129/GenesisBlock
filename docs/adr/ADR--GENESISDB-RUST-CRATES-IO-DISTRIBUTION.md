---
status: accepted
owner: GenesisBlockDB Engineering
date: 2026-09-04
issue: 166
---

# ADR — Rust / crates.io Distribution Boundary

## Context

GenesisBlockDB's root Rust package (`genesis-block-native`) is currently the implementation core for several product surfaces at once:

- the embedded storage/query engine;
- the Node.js N-API addon (default feature);
- the standalone Axum server (`bins` feature);
- C FFI/static library output;
- Android JNI/mobile builds;
- integration tests and benchmark binaries.

`Cargo.toml` intentionally contains `publish = false`.

Publishing this root crate to crates.io today would create a semver promise around a broad implementation-facing API before a stable Rust consumer boundary has been defined. It would also make N-API the default feature of a nominally general Rust database crate, which is the wrong default for ordinary Rust consumers.

## Decision

**Do not publish the current root `genesis-block-native` crate to crates.io.**

For the current product line, supported Rust paths remain:

1. source checkout + Cargo for contributors/advanced integrators;
2. standalone server binaries/containers for language-neutral consumers;
3. published ecosystem bindings for Node/mobile clients.

A crates.io package becomes eligible only after a deliberately public Rust boundary exists.

## Required gate before crates.io publication

One of the following designs MUST be completed before publication:

### Option A — extract a public crate

Create a consumer-facing crate (for example `genesisblockdb` or `genesisblockdb-core`) with:

- a small documented public API;
- no N-API dependency in default features;
- semver compatibility policy;
- package metadata (`repository`, `license`, `readme`, categories/keywords);
- explicit `include`/`exclude` policy;
- docs.rs build coverage;
- clean external consumer tests from crates.io-compatible packaging.

### Option B — stabilize the root crate as the public crate

Only if the root package is intentionally made the public Rust API:

- default features MUST represent ordinary Rust usage, not Node.js bindings;
- N-API, server binaries, mobile/JNI and benchmark-only dependencies MUST remain optional and feature-gated;
- public API modules MUST be reviewed for semver stability;
- `cargo package` and a clean unpacked-crate consumer MUST pass in CI.

## Acceptance criteria for future publication

Before changing `publish = false`:

- [ ] crate name availability/ownership has been verified;
- [ ] `cargo package` succeeds from a clean checkout;
- [ ] packaged contents contain all required runtime/license/readme files and no accidental large/internal artifacts;
- [ ] a clean external Cargo project can depend on the packaged crate without a local path dependency;
- [ ] default features compile without Node.js/N-API tooling;
- [ ] public API documentation defines supported vs internal modules;
- [ ] version compatibility policy is documented;
- [ ] CI exercises the exact publishable artifact.

## Consequences

### Positive

- avoids prematurely freezing internal engine APIs;
- avoids presenting Node-oriented defaults as a general Rust developer experience;
- keeps crates.io documentation honest: **planned/decision-resolved, not published**;
- allows the server/container distribution to mature independently.

### Negative

- Rust consumers cannot yet use `cargo add genesisblockdb` from crates.io;
- source/path/git dependencies remain the only direct Rust embedding route until a public crate boundary is introduced.

## Rejected shortcut

Simply changing:

```toml
publish = false
```

to a publishable setting is explicitly rejected. Distribution is an API/support contract, not a registry checkbox.
