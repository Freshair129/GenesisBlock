# `genesis-block-native` — Phase 3.1 scaffold

The Rust crate that the future Genesis Block backend ships as. This directory holds **only** the scaffold defined by `BLUEPRINT--GENESIS-GRAPH-INTEGRATION` phase **P3.1**:

- A `cdylib` Cargo crate produced by `napi-rs`.
- Two trivial sync exports — `schemaVersionSync()` and `engineNameSync()` — matching the two blocking calls allowed by `PROTOCOL--GENESIS-GRAPH-FFI` §5.
- A CI matrix (`.github/workflows/rust.yml`) that builds the crate on Linux / macOS / Windows.

Storage primitives (`addNode`, `addEdge`, `query`, `neighbors`, `cypher`) land in **P3.2 – P3.4**. Until then this crate is intentionally a no-op.

## Toolchain prerequisites

- **Rust ≥ 1.80** with the host's default target (install via `rustup`).
- **Node ≥ 20** with `@napi-rs/cli` (installed via `npm install` in this directory).
- **Platform linker**:
  - Linux: `gcc` (Ubuntu: `build-essential`).
  - macOS: Xcode Command Line Tools.
  - Windows: MSVC Build Tools 2019+ (the `x86_64-pc-windows-msvc` target requires `link.exe`).

## Building locally

```sh
cd packages/gks/native/genesis-block
npm install
npm run build          # release; produces ./genesis-block.<triple>.node
npm test               # runs __test__/sanity.test.mjs against the .node
```

`cargo test` covers the pure-Rust units in `src/lib.rs`.

## What's intentionally NOT here yet

| Item | Tracked in |
|---|---|
| Storage page layout | BLUEPRINT P3.2 |
| Bi-temporal columns | BLUEPRINT P3.3 |
| Cypher v0 parser/executor | BLUEPRINT P3.4 (mirrors `packages/gks/src/memory/graph/cypher-v0.ts`) |
| Benchmarks vs `GraphStore` | BLUEPRINT P3.5 |
| Default-flip in `MemoryStore` | ADR amendment per `ADR--GENESIS-GRAPH-AS-GKS-BACKEND` §5 |
| `linux-x64-musl` / `linux-arm64-gnu` cross-compile | P3.5 (CI matrix expansion) |
