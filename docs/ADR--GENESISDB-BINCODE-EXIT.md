# ADR — GENESISDB-BINCODE-EXIT

- **Status:** Proposed (2026-07-19)
- **Driver:** RUSTSEC-2025-0141 — bincode 1.3.3 unmaintained (team ceased development 2025-12). Currently ignored in `.github/workflows/security.yml`; this ADR replaces the ignore with an exit plan.
- **Deciders:** Boss (merge gate), Claude Fable 5 (author)

## Context

`bincode` is used in exactly one subsystem: per-collection metadata snapshots (`meta_<name>.bin`) — `bincode::serialize(&Vec<NodeMetadata>)` on save ([src/lib.rs:4492]) and a **try-chain deserialize** on load: `Vec<NodeMetadata>` (V1) → fallback `Vec<NodeMetadataV0>` (legacy) ([src/lib.rs:4743-4777]). The WAL and `state.json` do not use bincode. The blob is non-self-describing (noted at src/lib.rs:436), which is why the try-chain pattern exists.

An unmaintained, non-self-describing binary deserializer sitting on the persistence path of a database is unacceptable long-term: no fixes will ship if a deserialization CVE or a Rust-version incompatibility lands.

## Decision

**Adopt `postcard` (serde, maintained, no_std-compatible) for metadata snapshots; keep bincode as a read-only legacy fallback for a deprecation window.**

1. **Write path:** always serialize `Vec<NodeMetadata>` with `postcard::to_allocvec`. Prepend the 4-byte magic `b"GBP1"` so future formats are sniffable (fixes the non-self-describing pain permanently — bincode-era blobs have no magic, which is itself the discriminator).
2. **Read path** extends the existing try-chain, sniffing first:
   - magic `GBP1` → postcard `Vec<NodeMetadata>`
   - no magic → bincode `Vec<NodeMetadata>` → bincode `Vec<NodeMetadataV0>` (both legacy, unchanged code)
3. **Compaction/save rewrites** any legacy-loaded snapshot in the new format on the next `save_state()` (same behavior the schema-version code already documents at src/lib.rs:1997-2006). `SCHEMA_VERSION` stays at 1 — the magic byte, not the schema counter, discriminates the container format; node/edge shapes are unchanged.
4. **Deprecation window:** bincode stays as a dependency (read-only) for ≥2 minor releases after this ships, then the legacy arms and the dependency are removed and the RUSTSEC ignore deleted from security.yml.

## Alternatives rejected

- **bincode 2.x:** same maintainership situation that triggered the advisory; migrating to a differently-shaped API of the same abandoned org buys nothing.
- **rmp-serde / MessagePack:** self-describing but larger blobs and slower; self-description is redundant once the magic prefix exists.
- **serde_json:** human-debuggable but ~4-8× size on metadata vectors; metadata blobs are the hot save/load path at 1M nodes.
- **Do nothing (keep the CI ignore):** silent risk accrual on the persistence path; ignores are for transitives you can't control, not direct deps you can.

## Consequences

- `postcard` aligns with the `mobile` feature direction (no_std-capable).
- Fresh DBs created after this change are unreadable by older engine builds (forward-incompat, standard for a format bump; the magic makes the failure loud, not silent garbage).
- Tests required: postcard roundtrip; legacy bincode V1 and V0 fixtures still load (golden files); legacy snapshot is rewritten as postcard after save; mobile build (`--no-default-features --features mobile`) still compiles.

## Verification gates

```
cargo test --no-default-features --test schema_version_tests
cargo test --no-default-features --test persistence_tests
cargo test --no-default-features --test sidecar_migration_tests
cargo build --no-default-features --features mobile
```
