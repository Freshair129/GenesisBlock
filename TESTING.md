# TESTING.md — GenesisBlockDB Reliability Test Suite

## Test philosophy

Benchmark reports and competitive audits provide evidence of *current* behavior, but they are point-in-time snapshots. Automated regression tests prevent *future* breakage. Every critical engine behavior — storage durability, vector indexing, graph traversal, bitemporal correctness, API contracts — should have a deterministic test that runs on every commit.

This suite converts historical audit findings (HNSW-not-rehydrated, edge-panic, limit-ignored, string-identity drift) into automated regression tests that guard against recurrence.

## Test matrix

| Category | File(s) | Approx. count | What it covers |
|---|---|---|---|
| Storage/WAL | `tests/storage_reliability.rs` | 13 | open/close/reopen, WAL replay, snapshot reload, read-only, duplicate IDs, status |
| Vector/HNSW | `tests/vector_collections.rs` | 14 | search, dim mismatch, multi-collection isolation, recall@1k, snapshot rehydrate, ef_search |
| Graph traversal | `tests/graph_traversal.rs` | 15 | depth, direction, rel filter, cycle safety, limit, retract, fanout, path tracking |
| Bitemporal | `tests/bitemporal.rs` | 10 | supersede, as_of, caused_by, logical clock, TTL, edge retract temporal |
| HQL | `tests/hql.rs` | 8 | valid/invalid queries, unicode, quoted IDs, depth 0, context tier |
| REST API | `tests/rest_api.rs` | 13 | all major routes via tower::ServiceExt, malformed input, body limits |
| Concurrency | `tests/concurrency.rs` | 6 | concurrent reads/writes, bulk ops, search-while-write |
| Robustness | `tests/robustness.rs` | 12 | unicode/Thai/emoji, large props, bad dims, empty IDs, special chars |
| JIT/chunk schema | `tests/jit_chunk_schema.rs` | 7 | chunk node props, source pointers, document hierarchy, content hash |
| MCP tools | `__test__/mcp.test.mjs` | 8 | tool listing, add_knowledge, query_hql, context retrieval, errors |
| NAPI bindings | `__test__/sanity.test.mjs` | 3 | engineName, version, schemaVersion |

**Total: ~100+ deterministic automated test cases**

## Commands

```bash
# Full Rust test suite (core engine, no NAPI symbols — works on all platforms)
cargo test --no-default-features

# Rust tests with all features (requires NAPI toolchain)
cargo test --all-features

# Single test file
cargo test --test storage_reliability

# Single test by name substring
cargo test --test storage_reliability -- wal_durability

# Node.js tests (requires native addon build first)
npm run build        # or npm run build:debug for faster iteration
npm test

# Formatting check
cargo fmt --check

# Lint
cargo clippy --no-default-features -- -D warnings

# Benchmarks (manual, not in CI gate)
cargo bench --bench ldbc_lite
cargo run --release --features bins --bin industrial-audit
```

## What is intentionally NOT in normal CI

| Category | Reason |
|---|---|
| Head-to-head benchmarks (Chroma, Qdrant, Neo4j, Kuzu) | Require external services, long runtime |
| Large-scale 1M vector tests | ~11 GB RAM, multi-minute runtime |
| Real embedding model calls (bge-m3, OpenAI) | External API dependency |
| Heavy stress tests (100k+ nodes) | CI resource constraints |
| Criterion benchmarks | Measured separately in `benchmarks.yml` |
| Network-dependent tests | Gossip/sync over real sockets |

## Test data guidelines

- All vector tests use **deterministic synthetic vectors** (small dimensions, e.g. 4 or 8).
- All databases use **temporary directories** (`env!("CARGO_TARGET_TMPDIR")`).
- No timing assertions — test correctness, not latency.
- No external network calls or real embedding APIs.
- Keep normal test data small (< 1000 items per test).

## Future work

- **Property-based tests** with `proptest` for HQL parser and node/edge roundtrips
- **Fuzzing** the HQL parser with `cargo-fuzz` / AFL
- **Crash simulation** for WAL/snapshot (kill-during-write recovery)
- **Long-running concurrency soak tests** (hours, not seconds)
- **Benchmark CI job** with Criterion + regression detection
- **Rebuild-from-source-of-truth tests** (PostgreSQL → GenesisBlockDB)
- **Jepsen-like distributed consistency tests** if CRDT sync becomes production scope
- **JIT resolver contract tests** with pointer resolution + token-budget constraints

## JIT / chunk pointer schema

GenesisBlockDB stores chunk nodes as regular nodes with specific labels (`Chunk`, `DocumentChunk`) and props (`source_type`, `source_table`, `source_id`, `document_id`, `chunk_index`, `chunk_strategy`, `token_count`, `content_hash`, `title_path`). The engine provides the persistence, indexing, and graph traversal substrate — it does **not** own chunking logic or source-of-truth resolution.

A separate JIT resolver is responsible for:
1. Resolving `source_type` + `source_table` + `source_id` pointers to live content
2. Token-budget-aware context assembly
3. Chunk invalidation when the source changes
4. Content-hash deduplication (if desired — the engine stores both duplicates)

The `tests/jit_chunk_schema.rs` tests validate that the engine faithfully preserves all chunk metadata through save/reopen cycles and supports the expected document hierarchy graph patterns.
