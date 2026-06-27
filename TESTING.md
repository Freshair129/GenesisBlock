# TESTING.md — GenesisBlockDB Reliability Test Suite

## Test philosophy

Benchmark reports and competitive audits provide evidence of *current* behavior, but they are point-in-time snapshots. Automated regression tests prevent *future* breakage. Every critical engine behavior — storage durability, vector indexing, graph traversal, bitemporal correctness, API contracts — should have a deterministic test that runs on every commit.

This suite converts historical audit findings (HNSW-not-rehydrated, edge-panic, limit-ignored, string-identity drift) into automated regression tests that guard against recurrence.

## Test matrix

48 Rust test files under `tests/`, plus Node.js test files under `__test__/`.

| Category | File(s) | Count | What it covers |
|---|---|---|---|
| Storage/WAL | `storage_reliability.rs`, `persistence_tests.rs`, `wal_compaction_tests.rs` | 16 | open/close/reopen, WAL replay, snapshot reload, read-only, duplicate IDs, status, WAL compaction |
| Vector/HNSW | `vector_collections.rs`, `async_indexing_tests.rs`, `hnsw_capacity_tests.rs`, `ef_search_tests.rs`, `collection_ef_default_tests.rs`, `add_vector_tests.rs` | 33 | search, dim mismatch, multi-collection isolation, recall@1k, snapshot rehydrate, ef_search, async indexing, capacity, add_vector |
| Quantization | `quantization_tests.rs`, `binary_quant_tests.rs`, `rerank_tests.rs` | 13 | SQ8/BQ quantization, rerank accuracy |
| Graph traversal | `graph_traversal.rs`, `neighbors_direction_rels_tests.rs` | 24 | depth, direction, rel filter, cycle safety, limit, retract, fanout, path tracking |
| Bitemporal | `bitemporal.rs`, `temporal_queries_tests.rs`, `retract_edge_tests.rs` | 16 | supersede, as_of, caused_by, logical clock, TTL, edge retract temporal |
| HQL | `hql.rs`, `hql_collection_tests.rs` | 14 | valid/invalid queries, unicode, quoted IDs, depth 0, context tier, collection scoping |
| HQL fuzz | `hql_fuzz_tests.rs` | 34 | ~5000+ inputs: random bytes, unicode stress, SQL injection, boundary values, zero panics |
| REST API | `rest_api.rs`, `rest_api_tests.rs` | 40 | all routes via tower::ServiceExt, malformed input, body limits, SDK body shapes |
| Graph/edges | `edge_u128_tests.rs`, `edge_interning_tests.rs` | 12 | u128 edge keys, edge id interning, legacy migration |
| Node/identity | `node_interning_tests.rs`, `node_meta_a2_tests.rs`, `ephemeral_nodes_tests.rs` | 7 | node id interning, metadata, TTL/ephemeral |
| Multi-collection | `multi_collection_tests.rs` | 6 | per-collection isolation, cross-collection ops |
| Governance | `governance_tests.rs` | 1 | tier enforcement, MASTER-tier guard |
| CRDT/sync | `crdt_sync_tests.rs`, `consensus_commit_tests.rs`, `consensus_vote_sig_tests.rs`, `merkle_convergence_tests.rs`, `anti_entropy_tests.rs` | 22 | LWW reconciliation, signed votes, Merkle roots, anti-entropy |
| GRL/context | `grl_retrieval_tests.rs` | 3 | tiered context retrieval, hop budgets |
| Crash simulation | `crash_simulation_tests.rs` | 15 | truncated/corrupt WAL, missing/corrupt snapshot, double crash, vector search after WAL recovery |
| Rebuild from truth | `rebuild_from_truth_tests.rs` | 10 | snapshot/WAL round-trip for nodes, edges, props, vectors, graph indices, 1000-node batch |
| Soak (manual) | `soak_tests.rs` | 2 | sustained ingest→query→compact cycles (light 12s, medium 5min); `#[ignore]`d |
| Concurrency | `concurrency.rs` | 6 | concurrent reads/writes, bulk ops, search-while-write |
| Robustness | `robustness.rs`, `thai_fuzzy_tests.rs`, `hardening_tests.rs` | 16 | unicode/Thai/emoji, large props, bad dims, empty IDs, special chars, fuzzy matching |
| Batch/compaction | `batch_atomicity_tests.rs`, `compaction_tests.rs` | 2 | batch atomicity, index compaction |
| Schema/state | `schema_version_tests.rs`, `state_transition_tests.rs` | 4 | schema versioning, state transitions |
| Core engine | `core_engine_tests.rs` | 4 | end-to-end engine smoke tests |
| JIT/chunk schema | `jit_chunk_schema.rs` | 7 | chunk node props, source pointers, document hierarchy, content hash |
| MCP tools | `__test__/mcp.test.mjs` | 8 | tool listing, add_knowledge, query_hql, context retrieval, errors |
| NAPI bindings | `__test__/sanity.test.mjs` | 3 | engineName, version, schemaVersion |

**Total: ~300+ deterministic automated test cases across 48 Rust + 2 Node.js test files**

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
- **Jepsen-like distributed consistency tests** if CRDT sync becomes production scope
- **JIT resolver contract tests** with pointer resolution + token-budget constraints

### Done (moved from future work)

- ~~Fuzzing the HQL parser~~ → `hql_fuzz_tests.rs` (34 tests, ~5000+ inputs)
- ~~Crash simulation for WAL/snapshot~~ → `crash_simulation_tests.rs` (15 tests)
- ~~Long-running soak tests~~ → `soak_tests.rs` (light 12s + medium 5min, `#[ignore]`d)
- ~~Benchmark CI job~~ → `.github/workflows/bench-manual.yml` (manual trigger, soak + Criterion + audit bins)
- ~~Rebuild-from-source-of-truth tests~~ → `rebuild_from_truth_tests.rs` (10 tests)

## JIT / chunk pointer schema

GenesisBlockDB stores chunk nodes as regular nodes with specific labels (`Chunk`, `DocumentChunk`) and props (`source_type`, `source_table`, `source_id`, `document_id`, `chunk_index`, `chunk_strategy`, `token_count`, `content_hash`, `title_path`). The engine provides the persistence, indexing, and graph traversal substrate — it does **not** own chunking logic or source-of-truth resolution.

A separate JIT resolver is responsible for:
1. Resolving `source_type` + `source_table` + `source_id` pointers to live content
2. Token-budget-aware context assembly
3. Chunk invalidation when the source changes
4. Content-hash deduplication (if desired — the engine stores both duplicates)

The `tests/jit_chunk_schema.rs` tests validate that the engine faithfully preserves all chunk metadata through save/reopen cycles and supports the expected document hierarchy graph patterns.
