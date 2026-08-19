# Changelog

All notable changes to GenesisBlockDB are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — WP-3.2 G3 moat bench + PROCEED verdict

- **`moat-bench`** (`benches/moat_bench.rs`, bins-gated `[[bin]]`;
  wrappers `benchmark/run_moat_bench.{sh,ps1}`): the engine's fused
  vector+graph+AS-OF jobs vs the DIY single-SQLite-file assembly ROUND2
  named as the primary embedded competitor (brute f32 scan =
  sqlite-vec-stable model, recursive-CTE hops, shared Rust RRF glue,
  audit-history temporal pattern). Both sides in-process in one Rust
  binary — reported wins are lower bounds. Deterministic seeded corpus,
  clone-and-run, trust-gated through `verify_report.py`.
- **Verdict: PROCEED** (`docs/REPORT--G3-MOAT-VERDICT.md`, consumed by the
  WP-3.3 decision gate). At 100k×1024: Q1 fused 187.9×, Q3 114.9×,
  controls 92×/83.9× — every cross-dimension query clears the ROUND2
  G3-e ≥5× bar by an order of magnitude, and the advantage grows with
  dimension span (Q1 > Q3 > controls). The baseline structurally fails
  2/5 WP-3.1 bitemporal correctness scenarios (no tx axis; no provenance
  identity). Disclosed honestly: ingest stays the engine's weak side
  (141.9 s vs 33.1 s bulk), Q2 skipped until the FTS axis (S3) ships,
  synthetic corpus carries no recall claim.

### Added — WP-3.1 bitemporal correctness suite

`tests/bitemporal_matrix_wp31_tests.rs` — the correctness matrix the DIY
SQLite assembly must also pass in the moat bench (interview ROUND2 G3-e bar;
GNSE plan Phase 3 "Prove or kill"). Tests only, no engine changes:

- **valid×tx four-quadrant matrix** on a superseded node — including the
  two-axis case: "at commit S1 the recorded belief about 2022 had an OPEN
  validity window" vs today's closed one.
- **Retraction across tx time** — current view and at-or-after beliefs drop
  the node. The belief-BEFORE half is a deliberate `#[ignore]`d TDD RED
  test: the disclosed `implemented_post_resolution` semantics (WP-2.2)
  cannot resurrect a retracted node from current indexes; un-ignore when
  epoch-segmented indexes land (WP-3.3 gate). Not rewritten to assert the
  gap as expected behavior.
- **Correction-after-the-fact** — a retroactive `retract_edge` changes the
  answer to the same valid-time question across tx time.
- **Interval-overlap boundaries** — `valid_from <= as_of < valid_to`
  (start inclusive, end exclusive) probed at all four boundary points.
- **Audit reconstruction** — create → 2 supersessions → retract fully
  reconstructed from the `node_versions` chain; the WP-2.3 `caused_by`
  auto-chain walked backwards v3→v2→v1 purely from stored identities.
- **Reopen survival** — chain length and both temporal axes re-verified
  after a process restart.

### Added — WP-2.3 caused_by auto-chain + queryable recorded_at

- **`caused_by` auto-chain on supersede** (`supersede_node`): when the caller
  passes no provenance, the new version's `caused_by` now defaults to the
  identity of the version the supersession closed — `<id>@<frame_seq>` of
  the closing frame — instead of staying empty. The embedded frame seq
  resolves that exact version back through the WP-2.1 `node_versions`
  tx-time chain, so every unannotated supersession is self-documenting.
  An explicit caller-provided `caused_by` always wins, unchanged.
- **`recorded_at` queryable in HQL pattern clauses**: `qual_tail` (grammar
  `src/query/hql.pest`, kept in sync with `src/query/ast.rs`) gains a
  `recorded_at` accessor — `e.recorded_at` works in WHERE, ORDER BY, and
  RETURN of `MATCH` patterns. Edge bindings project their tx-time ingestion
  timestamp (RFC3339, so string comparison is chronological); node bindings
  resolve to null (NodeOutput carries no `recorded_at`), mirroring the
  `score`/`depth` convention. First time tx-time is reachable from query
  text rather than only via the `node_versions` API.

New suite: `tests/wp23_semantics_tests.rs` (auto-chain resolves through the
version chain, explicit provenance wins, RETURN projection, WHERE filtering
incl. the null-on-node case).

### Added — WP-2.2 tx_as_of + the as_of semantics fix

- **`temporal.tx_as_of` on the Typed Query IR** (search + traverse; NAPI,
  REST, and new FFI/JNI surfaces `genesisdb_execute_query_ir` /
  `genesisdb_query_ir_capabilities` + JNI mirrors; `include/genesisdb.h`
  regenerated): a replica-local commit-seq selector — "what did this replica
  believe at its commit N". Selectors below `history_horizon()` fail
  explicitly with `beyond_horizon` (ADR D4 rule 2). Interim semantics,
  disclosed by capabilities as `tx_as_of: "implemented_post_resolution"`:
  candidates come from current indexes, then each result node is re-resolved
  through the WP-2.1 version chain at N — nodes with no committed version
  at-or-below N, or retracted at N, are dropped (epoch-segmented indexes
  remain gated GNSE backlog).
- **`as_of` (valid-time) semantics fix** in `hybrid_search` and `neighbors`:
  a node whose current version postdates the selector now resolves its
  historically valid version from the chain (with its closed validity
  window) instead of being silently hidden — closing the superseded-node
  defect the GNSE review flagged, which `temporal_queries_tests` had
  codified as expected behavior (that assertion is now inverted). Chain
  lookup runs only when the current version fails the window (cold path);
  below the fold horizon the node stays hidden, matching the disclosed
  retention forfeit.

New suite: `tests/tx_as_of_wp22_tests.rs` (historical resolution,
not-yet-committed drop, beyond-horizon rejection, superseded-version
resolution on the search path).

### Added — WP-2.1 node_versions (tx-time version chain)

The first queryable tx-time surface (GNSE plan Phase 2): a per-entity version
chain in the SQLite projection, keyed by the LOCAL frame seq (ADR D2 —
replica-local commit order; PROJECTION_SCHEMA_VERSION 2 → 3, additive
migration).

- Every committed `Node` frame appends a chain row (deliberately NOT
  clock-LWW-gated — the chain records what was committed, in frame order);
  `NodeRetract` frames append retraction markers, so resolve-at-commit past a
  retraction answers "retracted", not the last live version. Supersede
  naturally yields close+new row pairs.
- Read API `node_versions(id, at_seq?)` on Storage, NAPI (`nodeVersions`),
  and REST (`GET /v1/node/versions?id=..&at_seq=..`): frame-ordered chain +
  optional resolve-at-commit. Lookup is by id string, so a retracted node's
  chain stays addressable after its interning entry is gone.
- **ADR D4 enforced:** rows below `history_horizon()` are never served (a
  projection rebuild would not recover them — the chain stays strictly
  rebuildable, proven by test), and `at_seq` below the horizon fails
  explicitly with `beyond_horizon` — never silently the current state. Under
  `frontier_only` this means the chain collapses at every fold, exactly the
  forfeit the capabilities surface discloses; under `full`/`budget` (WP-1.3)
  real history accumulates.

New suite: `tests/node_versions_wp21_tests.rs` (chain shape across
supersede, resolve-at-commit, retraction resolution, journal rebuild
identity, beyond-horizon behavior after a fold).

### Added — WP-1.3 retention profiles (ADR D3)

Journal retention is now a per-database setting: `OpenOptions.retention`
(`"frontier_only"` | `"full"` | `"budget:<bytes>"`; REST server env
`GENESIS_RETENTION`; unrecognized values fail `open` loudly — no
silent-default trap).

- **`frontier_only`** (default, unchanged behavior): fold at every
  checkpoint; forfeits tx-time history — and the capabilities surface now
  says so.
- **`full`**: checkpoints never fold; history accumulates as sealed segments
  and the journal retains full post-adoption history. Explicit `compact()`
  still folds.
- **`budget:<bytes>`**: checkpoints fold only when sealed history exceeds the
  byte budget — the bounded-disk contract, retaining up to the budget of
  tx-time history between folds. The active-file seal threshold derives from
  the budget (N/4, clamped to [64 KiB, 64 MiB]) so small (mobile-sized)
  budgets actually seal and trip. Interim semantics: a tripped budget folds
  the whole history window (the ADR's oldest-first partial fold needs a
  state-as-of-boundary materializer — deferred), which still bounds disk.
- **Horizon honesty (ADR I6, previously unreachable from any surface):**
  `query_ir_capabilities` gains `temporal.{history_horizon, tx_epoch_start,
  retention_profile, tx_time_retention}`; `GET /v1/frontier` gains
  `history_horizon` + `retention_profile` (additive); new NAPI
  `historyHorizon()`.
- Tombstone GC (Slice 1) now also runs on non-folding checkpoints, so the
  registry/snapshot stay bounded under `full`/`budget`.
- Deferred, per plan: default flip to `budget:4GiB` (belongs with the WP-2.x
  tx-time landing), the peer-aware retention floor (the sync commit-seq
  cursor is not yet used by requesters), and the archive hook.

New suite: `tests/retention_wp13_tests.rs` (fail-loud parsing, per-profile
fold behavior, bounded-disk under churn with journal-only recovery, horizon/
retention disclosure).

### Added — Slice-1 tombstone retention

Closes the two documented Slice-0 residuals (deletion convergence and the
fold's destruction of retraction history), within an interim 30-day retention
window (`TOMBSTONE_RETENTION_SECS`; policy moves to WP-1.3 retention profiles):

- **Node tombstone registry** (`Storage.tombstones`): every retraction records
  `{clock, retracted_at}`, persisted in `state.json` and re-emitted into the
  fold payload as `NodeRetract` frames — so the deletion survives snapshots,
  folds, and journal-only recovery.
- **CRDT deletion convergence:** `reconcile_state` now gates `Node` upserts by
  tombstone LWW — a stale peer re-push can no longer resurrect a retracted
  node after the origin folds (previously guaranteed resurrection: no local
  copy remained to win LWW). Remote `NodeRetract` events are recorded even
  when no local node is resident, with clock-idempotent re-offer handling.
  A genuinely newer upsert clears the tombstone (legitimate re-create).
- **Retracted edges survive the fold** within the retention window, restoring
  `retract_edge`'s documented time-travel contract (`as_of` before the
  retraction / `include_invalid`) after a checkpoint when the journal is the
  only surviving copy. Retention comparisons parse RFC3339 (no lexicographic
  string compare); unparseable stamps are conservatively retained.
- **GC at the fold boundary:** tombstones and retracted edges older than the
  window leave the fold payload; expired tombstones are dropped from the
  registry and the snapshot. Known residual: a peer partitioned longer than
  the window can still resurrect a delete — WP-1.3 territory.
- `Event::NodeRetract` gains a `retracted_at` field (`serde(default)`;
  additive within the unreleased v3 format — no schema bump).

New regression suite: `tests/durability_slice1_tests.rs` (stale-push LWW,
fold/snapshot/journal-only tombstone survival, legitimate re-create,
retracted-edge time travel after fold, GC at the window).

### Fixed — Slice-0 durability (SCHEMA_VERSION 2 → 3)

Four acked-write-loss / resurrection defects from the 2026-08-19 storage-
readiness audit (RCA--SLICE0-DURABILITY). All four were silent — no test
injected I/O errors or crashed inside the checkpoint window.

- **Journal write errors are no longer swallowed.** The WAL writer thread
  previously discarded `write_all`/`flush` results and acked on `sync_all()`
  alone, so an ENOSPC/EIO frame was acknowledged as durable. The ack now
  requires the whole batch's write + flush + fsync to succeed, and after any
  I/O failure the writer is poisoned (every append refused with a failed ack)
  until a successful fold rebuilds a clean active file — a torn tail can no
  longer sit under later "successful" appends that replay would never reach.
- **`retract_node` is journaled.** Node retraction (including the hourly
  autonomic TTL/orphan prune) used to mutate memory only; a crash before the
  next checkpoint resurrected the node and its cascaded edges on replay, and
  CRDT peers re-pushed it. A new `Event::NodeRetract` frame is now persisted
  *before* the in-memory removal, replayed as a removal, applied to the SQLite
  projection (props/labels rows deleted, including on rebuild), replicated via
  `events_since` (clock-stamped), and applied with node-style LWW on
  `reconcile_state`. **This new frame kind is why SCHEMA_VERSION bumps to 3:**
  older engines silently skip unknown journal events — a downgrade would
  silently resurrect deletions — so it fails closed instead.
- **A checkpoint can no longer write a snapshot without its journal cursor.**
  If `build_compacted_wal()` failed, `save_state` used to write `state.json`
  with no `journal` cursor; the next open then skipped replay entirely,
  silently dropping every write acked after the save. `save_state` now aborts
  loudly on a failed payload build (the previous snapshot + full journal
  remain a complete recovery source), propagates `state.json` write errors,
  and the cursor-less recovery branch — still reachable for pre-frontier
  snapshots — now replays the full journal on top of the instant load
  (idempotent LWW; same one-time duplicate-arena-rows tradeoff as the legacy
  `wal_frontier` branch).
- **A stale snapshot older than a completed fold is no longer trusted.** In
  the crash window between `journal_fold` and the `state.json` rename, the old
  snapshot still holds state that was deleted and folded away; base-segment
  replay can only add, so recovery resurrected it. Open now detects
  `history_horizon() > snapshot frontier` and recovers from the journal alone
  (the base segment is a complete recovery source, invariant I8).

New regression suite: `tests/durability_slice0_tests.rs` (crash-image and
clean-reopen retraction survival, stale-snapshot-vs-fold, cursor-less
snapshot tail recovery).

### Changed — BREAKING (on-disk + API), WP-1.2 framed journal
- **On-disk journal format (SCHEMA_VERSION 1 → 2).** `genesis-graph.wal` (JSONL)
  is replaced by a framed journal: `wal/active.gwal` (GWA1 header + frames
  `[u32 len | u64 commit_seq | u32 crc32c | SignedEvent JSON]`) plus sealed,
  zstd-compressed `journal/*.gseg` segments. Frames wrap the **original**
  event bytes, so peer signatures now survive checkpointing (previously
  compaction re-signed every event with the local key). Migration is automatic
  and **one-way**: on first open the legacy WAL is sealed as segment 0
  (`kind=legacy_jsonl`, recovery-only). An engine older than the on-disk
  version now fails closed with `SCHEMA_VERSION_UNSUPPORTED` — never a partial
  read. See [ADR--GENESISDB-JOURNAL-HISTORY](docs/adr/ADR--GENESISDB-JOURNAL-HISTORY.md)
  and [SPEC--GENESISDB-JOURNAL-FORMAT-V1](docs/SPEC--GENESISDB-JOURNAL-FORMAT-V1.md).
- **Checkpoint folds instead of truncating.** `save_state()` now folds the
  journal into a base segment (live state) rather than rewriting the WAL file,
  so the journal remains a complete standalone recovery source at every instant
  and the seal is durable *before* the snapshot manifest advances (invariants
  I7/I9). Disk stays bounded exactly as before (interim `frontier_only`
  retention profile; budget profiles land in WP-1.3).
- **`stableFrontier()` / `GET /v1/frontier` semantics changed.**
  `stable_frontier` is now the **frame** frontier — the commit sequence of the
  last durable journal frame, advancing on *every* mutation. The previous
  meaning (sequence of the last transaction) is now `txnFrontier()`, and that
  is the value `GenesisTransaction.expected_frontier` must be CAS'd against.
  `GET /v1/frontier` returns `{"frame": N, "txn": M}` instead of a bare number;
  SDKs reading the old scalar should read `.txn`. `CommitResult.commit_sequence`
  is now the transaction's frame stamp.
- **Transaction sequences are replica-local.** `GenesisTransactionEvent.commit_sequence`
  is demoted to `origin_commit_seq` (serde alias keeps old WAL lines parsing);
  a replica no longer merges a peer's sequence into its own counter, which also
  removes a `applied_transactions` uniqueness collision that could abort a whole
  reconcile batch.

### Added
- **HQL Cypher-style graph patterns (path 1):** a fifth HQL command,
  `MATCH (a:Label {k:v})-[r:REL]->(b) ...`, matching linear path patterns by
  deterministic left-to-right expansion over the graph indices — **no query
  planner**. Supports node label/prop constraints, edge type + direction
  (`->`/`<-`/`-`), `{id:"…"}` anchoring, and variable-qualified
  `WHERE`/`ORDER BY`/`LIMIT`/`RETURN` (`a`, `a.id`, `a.label`, `a.prop.<key>`)
  plus `AS OF`. `MATCH (` routes to patterns; `MATCH <t> SIMILAR TO …` remains
  the hybrid command (no breaking change). v1 is linear-path-only (no
  variable-length `*`, branching, or `OR`). Lands on both NAPI (`executeHql`)
  and REST (`/v1/query/hql`) with no signature change. See
  `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md`; tests in
  `tests/hql_cypher_tests.rs`.

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
