---
doc_id: REPORT--HQL2-P8-CORE-2026-09-28
version: "0.1.0b"
created_at: "2026-09-28T04:35:00+07:00,ATHER,22bc11e"
last_update: "2026-09-28T06:08:00+07:00,ATHER"
status: beta
owner: GenesisBlockDB Engineering
attributes:
  domain: query-execution
  scope: P8 initial scalar Rust-core implementation checkpoint
  risk: HIGH
  complexity: C-3
---

# HQL2 P8 core checkpoint — not full HQL2 completion

## Authority and base

The owner approved P8 typed-boundary 0.1.1b and isolated upstream integration
with `approve` on 2026-09-28. Local merge `22bc11e` joins upstream `8091a56`
with preserved local P6 `fc851e9` and P7 `e20e0e4`. The primary checkout was not
changed. Protected `tests/zz_probe_discriminates.rs` was not read/changed/staged
or tested and is absent from the isolated checkout.
No fetch, push, main-branch merge, PR, deployment or user database migration.
Engine version 0.2.6 -> 0.2.9 is inherited from upstream, not a new release here.

## Implemented scope

- `src/query/hql2/`: full pinned grammar parser and spanned source tree; typed
  scalar lowering; all 23 closed operator configurations; iterative DAG checks;
  semantic scalar binder, deterministic physical plan, native typed kernels;
  safe errors and closed result encoding. No oracle imports in production.
- Exact runtime operators: Values, Filter, Project, Distinct, Sort, Take,
  Offset, UnionAll, Aggregate, Join (inner/left/semi/anti). Scalar/list parameter
  values are explicitly typed; I64 decimal-string encoding is lossless.
- `Storage::query_v2`: authorization before parsing, one continuous commit
  lock across catalog/bind/plan/pin/execute/final validation. EXPLAIN is catalog
  only and never invokes generation publication. Execution uses one real P6
  lease; snapshot identity derives only from the public verifying key.
- `ReadView::hql2_catalog` is private and lease-validated. Its borrowed result
  cannot escape the guard or expose Storage. The other three approved source
  methods are not implemented yet.
- Conservative query-local memory reservations, deadline, result-row/byte and
  collect bounds; exact execution fails on exhaustion, never returns partial
  aggregates. ANALYZE emits measured row/time counters only; unknown counters,
  costs and index frontiers remain absent.

These are Rust core changes only. REST/NAPI/FFI/SDK/MCP remain unchanged on
their existing compatibility contracts; P13 parity is deliberately not claimed.

## Verification record

Platform: local Windows MSVC, Rust 1.97.1, `--locked --offline
--no-default-features`, target directory
`G:/GenesisBlock_Dev/GenesisBlock/target/hql2-execution`.

| Gate | Measured result | Limit |
|---|---|---|
| Upstream integration prerequisites | 41 passed: G0/P6 36 + index coverage 1 + relational schema limits 4 | Local merge baseline only |
| P8 first integrated revision | 78 passed: storage 10, parser 13, request 5, scalar 27, value 5, wire 18 | Before last resource-review corrections; not final acceptance |
| Compatibility and preserved prerequisites | 306 passed: legacy HQL/IR 141 + G0/P6 36 + P7 129 | Exact scoped targets, not full suite |
| Final focused P8 scalar revision | 98 passed: storage 12, parser 18, lower 12, request 6, scalar 27, value 5, wire 18 | Final Error.reason source included; not full P8 |
| Independent review | No outstanding finding in reviewed scalar/resource boundary | No full P8 approval inferred |
| First full native Rust attempt | 142 passed, 1 failed before fail-fast stopped at epoch_e2 | Legacy snapshot fixture retained a stale P6 manifest; not a full-suite pass |
| Default-feature library check | cargo check --locked --offline --lib --jobs 1 passed | Final Error.reason field compiles with NAPI; no addon runtime claim |
| Final P8/P6 and corrected fixture rerun | 141 passed: P8 98 + P6 31 + epoch/metadata 12 | Default test profile; zero failures, allocator child entrypoint handled as described below |
| Full no-fail-fast native rerun | 913 passed, 1 failed, 4 ignored across 118 harness results; exit 1 | Sole failure: wal_tail_replay legacy fixture retained P6 while removing its required frontier; see correction below |
| Final WAL fixture/P6 rerun | 48 passed, 0 failed, 0 ignored; exit 0 | WAL 5 + epoch/metadata 12 + P6 31; production code unchanged since the full run; not a second full-suite invocation |
| Long-running soak | Three soak_tests cases remain ignored by the normal suite | No soak qualification inferred |
| Node / devices / hosted CI | Not verified in this record | No release, parity or performance claim |

Parser tests include all 35 positive and 6 negative vendored examples. The
parser harness reports one ignored child entrypoint, explicitly executed twice
by its passing parent (default/detailed diagnostic allocator probes). It is not
unrun coverage. The scalar differential test enumerates 81 small bags against independent P7
semantics for ordering/distinct and aggregates. Oracle-only graph/rank tests
remain fixture evidence, not storage-backed operator acceptance.

The existing REST integration tests run in this native suite. Their passing
results preserve the old surface only: no HQL2 transport is implemented or
qualified by those tests. No Node addon runtime test was run.

EXPLAIN storage tests compare signed journal, SQLite database/WAL and snapshot
bytes before/after fresh and changed stores, plus stable frontier and absent
actual counters. Windows `genesis.lock` and SQLite volatile `projection.sqlite-shm`
have presence/size checks instead of byte-content comparisons; neither is an
authoritative data/WAL artifact. Test mismatch diagnostics print names only.

Reproducible focused command from the isolated checkout:

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --test hql2_execution_tests --test hql2_parser_tests --test hql2_lower_tests --test hql2_request_tests --test hql2_scalar_execution_tests --test hql2_value_tests --test hql2_wire_tests --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-execution
```

First full-suite command (failed at the legacy fixture described above):

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-execution
```

No-fail-fast full-suite command (only this checkout, protected probe absent):

```powershell
cargo test --locked --offline --no-default-features --no-fail-fast --jobs 2 --config profile.test.debug=0 --config profile.test.debug-assertions=true --config profile.test.overflow-checks=true --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-qualification
```

This separate qualification target disables debug symbols only for this test
invocation, retains assertions and overflow checks, and does not edit Cargo
profiles or global configuration. It is not release-mode or performance evidence.

Final correction verification, with the same profile and target directory:

```powershell
cargo test --locked --offline --no-default-features --jobs 2 --config profile.test.debug=0 --config profile.test.debug-assertions=true --config profile.test.overflow-checks=true --test wal_tail_replay_tests --test epoch_e2_tests --test meta_format_migration_tests --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test p6_peer_authority_tests --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-qualification
```

Do not rewrite the preceding full run as a green full-suite invocation: its
failure was recorded before the final test-only correction. The lib-unit and
doc-test harnesses have zero tests; they are not extra acceptance evidence.

Documentation: Python 3.13 `-B scripts/validate_doc_status.py docs` reports zero
violations in 236 files. Scoped rustfmt and `git diff --check` pass. No transport
file changes and no HQL2/query_v2 route/binding in router/main/FFI/index.d.ts/MCP;
Rust internal-only surface selection is deliberate under the P8 contract.

## Review corrections and provenance

RCA files are local under `.brain/rca/` (ignored by default); this committed
report retains the causes and distinguishing evidence:

1. Plain HQL was mistaken for explicit explain=None; envelope Plan/Analyze
   incorrectly conflicted. Preserve omission and compare explicit modifiers.
2. Initial disk evidence collection tried reading the Windows ownership lock;
   OS error 33 preceded query execution. Volatile SQLite read marks also changed
   legitimately. Narrow only these two synchronization artifacts as above.
3. Early refusal of deeply constructed Rust JSON could recursively drop input.
   Retain ownership and dispose it iteratively; test 10000 nested arrays.
4. Collect evaded its element row bound; List equality borrowed an unregistered
   Rust comparator. Check cardinality and reject unsupported comparison at bind.
5. AVG initially bypassed checked same-type accumulation. The P7 discriminator
   AVG([i64::MAX,i64::MAX]) requires overflow, not a finite running-mean answer.
6. Recursive bind/eval/lower frames could exhaust a Windows-sized stack despite
   a logical depth cap. Use iterative traversals, with small-stack regressions.
7. Propagated aliases and per-expression type clones were uncharged. A 16KiB
   alias over 100 nodes exceeded a 1MiB reservation; deep types referenced 4000
   times exceeded a 3MiB reservation. Reserve these copies before binding.
8. G0 cloned long IDs before planning charges. Use borrow-only envelope checks
   and the mandatory closed IR decoder; reserve ID copies before validation.
9. Parser work and nested BETWEEN expansion require pre-allocation bounds, not
   a post-allocation node count. An iterative lexical preflight shares public
   parser/query-boundary limits: 262144 Unicode scalars, at most 2048 lexical
   units, and heap plus worker-stack reservation within 64 MiB. Some valid
   wide queries deliberately fail QUERY_BUDGET_EXCEEDED. The grammar/Pest-specific
   conservative bound is not an RSS measurement or a global allocator limit.
   Independent parser verification passed 18 tests, including two explicit
   child allocator probes. A 200033-byte wide rejection fell from 97518360
   to 709 incremental live allocation bytes. Lowering checks projected BETWEEN
   node count/depth before construction: depth25 refuses; small depth10 retains
   6139 wire expressions. Lowering passed 12 tests on the reviewed source.
10. A healthy read-only handle without a reusable current generation returned
    DATA_CORRUPTION because the P6 publisher's exact `read-only` reason hit the
    generic mapper. The runtime RED reproduced this classification after
    successful EXPLAIN. Map only that exact reason to CAPABILITY_UNSUPPORTED;
    do not prohibit existing-generation reuse or authorize publication writes.
    Use the shared Error.reason field: napi's Display adds a status prefix,
    unlike the native Error shim. Default-feature compilation is checked
    separately; no addon runtime or transport test is inferred from it.
11. Full-suite integration exposed old migration fixtures overwriting metadata
    while retaining the new P6 digest manifest. Epoch GBP1 migration failed;
    malformed metadata unexpectedly recovered through WAL, and the bincode
    test's search-only success was a false positive for decoder coverage.
    Correct only the test fixtures to schema4/no-P6 after asserting fresh
    Disabled revision0/no generation. Retain zero-epoch decoder evidence and
    add a modern manifest-corruption case requiring WAL recovery. Production
    snapshot validation is unchanged. Local RCA:
    `.brain/rca/RCA--HQL2-UPSTREAM-LEGACY-SNAPSHOT-FIXTURES.md`.
12. The no-fail-fast full run found the same fixture mismatch in WAL tail replay:
    `snapshot_without_frontier_still_loads` removed journal/wal_frontier but
    retained schema5/P6. Required frontier validation correctly rejected the
    snapshot; WAL replay produced 10 arena rows, not the expected legacy 20.
    Construct schema4/no-P6 only for the guarded legacy fixture, preserving its
    20-row assertion. Add a separate modern missing-frontier case requiring
    exactly 10 arena rows, all nodes and 10 search results after WAL recovery.
    Independent source review passed; production validation/recovery unchanged.
    Final targeted rerun passed all 48 tests, including both legacy and modern
    missing-frontier discriminators. No known failing tested case remains;
    the full suite was not rerun after this final test-only fixture change.

## Open work — do not promote P8/R1

Thirteen operators remain unavailable: NodeScan, EdgeScan, RowScan,
AnnotationScan, HistoryScan, ChangeScan, Match, Expand, AnnotationLookup, Knn,
Rerank, LexicalMatch and ContextPack. Existing record structs lack the new
uniform revision/source contract: NodeOutput and EdgeOutput carry logical
clocks, not HQL2 revision identities; node_versions is keyed by replica-local
frame sequence and node intern ID (`src/lib.rs`, node_versions schema and
projection_append_node_version). Existing node provenance uses id@frame, but coverage
after legacy replay/fold/rebuild still needs qualification. Edge history exists,
but `compact` rewrites surviving edge_versions.tx_from to the fold frontier;
that coordinate cannot silently become an immutable revision identity. Row
updates have no per-row revision chain; folded RelationalRows upserts have an
empty mutation_id, with idempotency receipts stored separately. Thus neither
batch IDs, SQLite rowids nor payload hashes supply the approved RecordRef
contract. These findings are not a claim that node/edge history is absent.
Annotation and row-history adapters,
model/analyzer/tokenizer identities and retained-domain proofs remain open.
Do not synthesize history or fingerprints from current payloads.

Remaining HQL scalar lowering includes contextual bare NULL/list/object
literals, parameterized limits, implicit NULL order and remainder; non-scalar
stages currently have syntax but no supported lowering. Legacy HQL remains on
its old runtime, not a falsely claimed shared-IR lowering. These are open tasks
within the approved target, not a reduced definition of done.

Cancellation handles, source work accounting, spill/scheduling/index lifecycle,
transport parity, durable extension/migration contracts, security qualification,
soak/crash/platform acceptance and the full 190-obligation ledger remain open.
No production source capability is inferred from enum/grammar presence.

The next authority gate is a concrete durable revision/annotation contract
under ADR H2-D11: issuance, update/delete/recreate identity, legacy/fold baseline,
retention and restore semantics. Typed adapters, ACL and pagination themselves
are already authorized; no new planner approval or automatic GBF2/GBO2 storage
rewrite follows from this gap. This record proposes no durable implementation.

## Version diff

| Artifact | Before | After |
|---|---|---|
| Engine (upstream) | 0.2.6 | 0.2.9 |
| P8 specification | 0.1.1b candidate | Owner-approved 0.1.1b, implementation-status patch 0.1.2b beta |
| P6 peer specification | 0.2.0b | 0.2.1b |
| Master architecture | 2.3.2b | 2.3.3b |
| C4 map | 0.1.14b | 0.1.15b |
| Registry | 0.4.8+draft | 0.4.9+draft |
| Orchestration plan | 0.2.2b | 0.2.3b |
| This report | absent | 0.1.0b |

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-28 | beta | Record approved integration, reviewed scalar/core checkpoint, full-run failure and green targeted corrections; retain incomplete P8/HQL2 gates | working-tree | ATHER |
