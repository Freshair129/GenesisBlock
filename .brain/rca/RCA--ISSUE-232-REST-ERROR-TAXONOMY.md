---
status: active
superseded_by: null
issue: 232
---

# RCA — API error taxonomy divergence (issue #232)

## Symptom

REST routes reported failures with different wire shapes. Most returned plain
text; Query IR and HQL budget failures returned ad hoc `{ "code", "message" }`
objects. SDKs therefore had no stable category/code contract to consume across
routes.

## Evidence

- `docs/SRS--GENESISBLOCKDB.md` requirement GB-SRS-API-004 requires stable
  machine-readable validation, capability, query, conflict, durability, and
  integrity categories.
- Before this correction, `src/router.rs` returned `(StatusCode, e.to_string())`
  from ordinary handlers while query handlers constructed their own JSON.
- Before this correction, `docs/API_REFERENCE.md` described plain-text REST
  errors and documented Query IR's `{ "code", "message" }` shape without a
  common envelope.
- Existing REST tests covered the route-specific legacy codes, but had no
  common category contract spanning representative routes.
- Python and Studio consumed message text differently; the Go client decoded
  only top-level `code` and `message`, so ordinary new-style errors would lose
  their machine code.
- `src/lib.rs` aliases core failures to `napi::Error` (or a local reason-only
  shim when NAPI bindings are disabled). Storage methods expose a diagnostic
  string, not a typed category.
- The HQL budget state also constructs `QUERY_BUDGET_EXCEEDED` and
  `QUERY_BUDGET_INVALID` as reason-only `Error::from_reason` values. The REST
  adapter had recovered status/code aliases by checking and splitting that
  diagnostic, so small message changes could alter the wire status/code.

## Root Cause

The REST layer grew handler by handler without a shared error response type or
extractor policy. The core error boundary also carries only a reason string,
so REST cannot reliably recover validation versus operational semantics from
the core without parsing user-facing diagnostic text. Existing query codes
were exposed as compatibility details, not a shared taxonomy.

## Why the issue escaped detection

The tests asserted successful route behavior and selected query-specific error
codes. They did not assert a common response envelope across routes, exercise
all required categories, or verify that each SDK reads the stable nested
fields. The API reference repeated the route-specific shapes rather than
documenting a single contract.

## Correction and prevention

1. Add a shared `ApiErrorEnvelopeV1` and stable category/code mapping used by
   REST and the additive embedded contract methods.
2. Normalize JSON, query-string, and path extractor rejections as validation
   errors while retaining their diagnostics and HTTP status.
3. Preserve Query IR's documented aliases; validate HQL budget input before it
   reaches the reason-only executor. For untyped HQL execution failures, keep
   the complete diagnostic but use the route's fixed `500` /
   `QUERY_EXECUTION_FAILED` compatibility mapping. Do not recover HQL runtime
   status or code by parsing diagnostic text. Query IR retains its old top-level
   `code` extraction as a compatibility alias only; stable category, stable
   code, and HTTP status now come from the typed request semantics.
4. Return durability only from the typed Storage recovery-required state.
5. Update Python, Go, and Studio remote clients to read nested envelope details
   while retaining their legacy fallbacks.
6. Add route- and SDK-level category coverage and keep the API reference, SRS,
   and this RCA aligned.

## Additive Core/NAPI contract and migration boundary

The additive type and first typed Storage/NAPI boundaries now exist:
`Storage::query_v2_contract` wraps structured `QueryErrorV2.code` values, and
`Storage::commit_transaction_contract` uses explicit transaction guards plus
the recovery-required state. `GenesisDatabase::queryV2Contract` and
`commitTransactionContract` return the versioned `{ contract_version, ok,
data|error }` object. The legacy `query_v2`, `commitTransaction`, and other
Storage/NAPI signatures remain unchanged; unknown reason-only Storage failures
map to `internal` only in the new contract.

The exact loss boundary remains for existing legacy methods: `Result<T>` is
backed by `napi::Error` with NAPI enabled or the reason-only shim when disabled.
Most older Storage methods and NAPI wrappers still return only message/reason;
REST handlers around them therefore keep explicit route fallbacks. The new
query boundary consumes `QueryErrorV2.code` as structured data and preserves
that code as a separate legacy alias. It does not inspect the message to
classify. `hql2_storage_error` remains a legacy adapter for reason-only Storage
failures; it is not used by the new envelope mapper to parse message text. The
HQL REST adapter likewise no longer parses `QUERY_BUDGET_*` diagnostics: known
invalid request limits are classified by the explicit preflight validator,
while runtime reason-only failures keep their diagnostic under the stable query
category and generic compatibility code. The runtime budget subcode is not
preserved as a top-level alias until Storage exposes that failure as typed data.
The Query IR REST adapter's only message-prefix-derived field is the deprecated
top-level `code` compatibility alias: it splits the diagnostic at the first
colon and preserves a `QUERY_*` candidate, otherwise using
`QUERY_EXECUTION_FAILED`. This does not affect HTTP status, nested
`error.category`, or stable `error.code`, which come from the request semantics
and shared taxonomy. Moving the legacy Query IR subcodes into structured
Storage results would allow that compatibility extraction to be removed without
dropping aliases.

### Six-category conformance matrix

| Category | Core/Storage source | NAPI surface | REST representative | SDK reader evidence |
|---|---|---|---|---|
| `validation` | `parse_query_v2_input`, request checks, relational identifier/row/primary-key preflight | `queryV2Contract`, `commitTransactionContract` | `ApiJson` rejection and Query IR request validation | Python/Go all-category fixtures |
| `capability` | structured `CAPABILITY_UNSUPPORTED` QueryErrorV2 code | `queryV2Contract` shared mapper | Unsupported Query IR operation | Python/Go all-category fixtures |
| `query` | structured `QUERY_BUDGET_EXCEEDED` code | `queryV2Contract` shared mapper | HQL budget exhaustion uses query category; runtime reason is retained with generic legacy alias | Python/Go all-category fixtures |
| `conflict` | transaction identity/frontier checks and SQLite extended primary-key/unique constraint codes from rollback-only relational preflight | `commitTransactionContract` | `/v1/transaction/commit` frontier mismatch, relational collision, or delete/write collision | Python/Go all-category fixtures |
| `durability` | explicit recovery state / post-WAL apply guard | `commitTransactionContract` | recovery-required write response | Python/Go all-category fixtures |
| `integrity` | structured `DATA_CORRUPTION` QueryErrorV2 code | `queryV2Contract` shared mapper | Consensus verification failure | Python/Go all-category fixtures |

Rust tests assert stable code/category/message serialization through the exact
NAPI result helper for all six categories, structured QueryErrorV2 mapping, and
actual Core transaction validation, conflict, and durability guards. The
transaction preflight maps relational identifier, row, and primary-key encoding
errors to validation by call-site context; missing schemas/collections to
not-found; dimension mismatch to validation; and governance policy rejection to
authorization. The REST transaction handler uses HTTP 403 for that governance
rejection. Under the transaction commit lock, the additive typed path applies
ordered relational mutations in a rollback-only projection transaction before
building or persisting the durable event. It classifies only SQLite's structured
`SQLITE_CONSTRAINT_PRIMARYKEY` and `SQLITE_CONSTRAINT_UNIQUE` extended codes as
conflicts; diagnostic text is retained but never parsed. This covers in-batch,
persisted, and delete-containing unique collisions, while delete-then-valid
replacement remains accepted. Other SQLite and persistence failures retain the
internal/durability mapping. NAPI runtime tests cover validation, structured
legacy query-code preservation, conflict, success, and legacy
Promise-rejection compatibility.
Query success serializes the inner `QueryResultV2` or `ExplainResultV2` payload
directly; the engine's non-serializable `QueryOutcomeV2` enum is neither changed
nor exposed as a JSON wrapper.
REST integration tests cover all six route categories, with a separate
recovery-required handler test. Python and Go SDK readers test stable and
legacy code separation for all six. Local Go execution remains unavailable
unless a Go toolchain or CI result is provided.

GB-SRS-API-004 parity is defined for the additive versioned contract entrypoints:
Core `query_v2_contract` / `commit_transaction_contract`, their NAPI wrappers,
the matching REST Query IR / transaction routes, and SDK readers of REST
responses. These surfaces share the same category/code mapping and have the
category-by-category evidence listed above. This satisfies the acceptance
criterion for equivalent failures where the v1 contract is exposed.

Legacy Storage/NAPI method signatures and reason-only errors remain unchanged
as the compatibility path. They are explicitly outside the v1 envelope contract
and are a separate migration boundary; this work does not claim typed parity for
every historical operation. Existing clients can keep using them, while clients
that need stable categories opt into the additive methods. This is the documented
migration path, not an unresolved blocker for the v1 contract. Issue #232 remains
open pending current-head validation and merge.

## Verification scope

The following passing results are from the earlier REST/SDK baseline at
`fb58a3e`; they predate the current additive Core/NAPI contract edits and must
not be presented as verification of the current working tree.

- `cargo check --no-default-features --features bins --bin genesis-db-server`:
  passed.
- `cargo test --no-default-features --test rest_api_tests`: 47 passed, 0 failed,
  including the REST category cases; durability was exercised by the separate
  handler test below.
- `cargo test --no-default-features --lib
  recovery_required_write_returns_durability_envelope`: 1 passed, 7 filtered;
  this exercises the recovery-required write route.
- `cargo build --no-default-features --features bins --bin genesis-db-server`:
  passed.
- Python client contract suite: 7 passed, including stable/legacy code fields.
- Studio transport tests: 5 passed; `tsc -b` and production `vite build` passed.
- Go client tests were added but could not be run: no Go executable is installed
  or available on PATH in this environment.
- `cargo fmt --all -- --check`: passed after moving the router test module below
  all non-test items.
- `cargo clippy --jobs 1 --no-default-features --all-targets -- -D warnings`:
  passed, including the test-only router module.
- The default-feature/NAPI all-target Clippy run was attempted with one Cargo
  job but could not finish because the worktree volume ran out of disk while
  writing `axum` metadata (`os error 112`); no NAPI lint result is claimed.

Current additive Core/NAPI changes are pending verification. Cargo was reserved
for serialized parallel work, so no Cargo command has been run against these
edits. The NAPI runtime tests now assert direct JS-object results and preserve
`VERSION_UNSUPPORTED` as a top-level compatibility code beside nested stable
`GENESIS_VALIDATION_ERROR`; this regression test has not yet run against a
rebuilt addon. The Go tests also remain unrun locally because the Go toolchain
is unavailable on PATH. Issue #232 stays open until current-tree Rust/NAPI
validation, Go test evidence (or exact-head CI), and merge.

The current follow-up edits pass standalone `rustfmt --check` and
`git diff --check`. Node syntax checks for the d.ts freshness gate and NAPI
contract test, the Python SDK suite (8 tests, using an in-memory `requests`
stub because the dependency is not installed), and the d.ts declaration count
and refined-signature anchors were also checked statically. The added Core
relational validation test remains unrun until the serialized Cargo slot is
released. REST/Cargo tests remain pending; Go tests remain pending a Go
toolchain.

## Cross-platform transaction conflict regression

The PR checks at the pre-fix head `fc7defc49b4c1b270d6087c7fa996fa91d648066` failed identically on Ubuntu, Windows, and the host build with mobile SDK features. In `rest_rejection_preserves_frontier_and_corrected_retry_succeeds`, two relational INSERTs with the same primary key returned HTTP 500 instead of 409. The typed transaction preflight had allowed the duplicate through to the projection, where SQLite's constraint error was converted to a reason-only Storage error and then mapped to `internal`.

The follow-up first added heuristic-free primary-key and insert-only unique-index preflight, which left mixed delete/write batches uncovered. The current patch replaces that classifier with a rollback-only projection transaction that executes the same ordered row mutations as the commit path and inspects SQLite extended constraint codes. The REST regression asserts HTTP 409 and nested stable `conflict` / `GENESIS_CONFLICT` for in-batch and persisted key collisions, persisted unique collisions, and a delete-containing unresolved unique collision; it also asserts that deleting the conflicting row before inserting a replacement succeeds. Rejected writes must preserve the frontier and rows. These latest source and test edits have only received standalone formatting and diff checks so far; Cargo validation and fresh CI are still required.

The Windows NAPI check at the prior head also expected the legacy
`commitTransaction` rejection for a duplicate primary key to match `/UNIQUE/`.
The existing regression in `__test__/wave_a.test.mjs` keeps that expectation,
checks that the frontier is unchanged, and retries successfully after correcting
the transaction. The new typed mapper retains SQLite's `UNIQUE constraint`
diagnostic in its legacy message, so the expectation should remain compatible;
the rebuilt-addon check is still pending at the updated head.
