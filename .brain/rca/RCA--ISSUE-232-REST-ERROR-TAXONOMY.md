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

## Additive Core/NAPI boundary and remaining limit

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
The Query IR REST adapter retains its pre-existing top-level compatibility
subcode extraction; stable category selection remains based on typed request
shape. Moving those legacy Query IR subcodes into structured Storage results is
still required before that extraction can be removed without dropping aliases.

### Six-category conformance matrix

| Category | Core/Storage source | NAPI surface | REST representative | SDK reader evidence |
|---|---|---|---|---|
| `validation` | `parse_query_v2_input`, request checks, relational identifier/row/primary-key preflight | `queryV2Contract`, `commitTransactionContract` | `ApiJson` rejection and Query IR request validation | Python/Go all-category fixtures |
| `capability` | structured `CAPABILITY_UNSUPPORTED` QueryErrorV2 code | `queryV2Contract` shared mapper | Unsupported Query IR operation | Python/Go all-category fixtures |
| `query` | structured `QUERY_BUDGET_EXCEEDED` code | `queryV2Contract` shared mapper | HQL budget exhaustion uses query category; runtime reason is retained with generic legacy alias | Python/Go all-category fixtures |
| `conflict` | transaction identity/frontier checks | `commitTransactionContract` | `/v1/transaction/commit` frontier mismatch | Python/Go all-category fixtures |
| `durability` | explicit recovery state / post-WAL apply guard | `commitTransactionContract` | recovery-required write response | Python/Go all-category fixtures |
| `integrity` | structured `DATA_CORRUPTION` QueryErrorV2 code | `queryV2Contract` shared mapper | Consensus verification failure | Python/Go all-category fixtures |

Rust tests assert stable code/category/message serialization through the exact
NAPI result helper for all six categories, structured QueryErrorV2 mapping, and
actual Core transaction validation, conflict, and durability guards. The
transaction preflight maps relational identifier, row, and primary-key errors
to validation by call-site context; missing schemas/collections to not-found;
dimension mismatch to validation; and governance policy rejection to
authorization. The REST transaction handler uses HTTP 403 for that governance
rejection. NAPI runtime tests cover validation, structured legacy query-code
preservation, conflict, success, and legacy Promise-rejection compatibility.
REST integration tests cover all six route categories, with a separate
recovery-required handler test. Python and Go SDK readers test stable and
legacy code separation for all six. Local Go execution remains unavailable
unless a Go toolchain or CI result is provided.

GB-SRS-API-004's additive typed query/transaction path now has category and
serialization evidence across Core, NAPI, REST, and SDK readers. The wider
legacy Storage/NAPI method surface has not been migrated to `ApiErrorEnvelopeV1`;
do not claim that every existing operation has typed category parity. Keep
issue #232 open until that boundary is accepted or explicitly scoped out.

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
for the serialized #230 suite, so no Cargo command has been run against these
edits. The NAPI runtime tests now assert direct JS-object results and preserve
`VERSION_UNSUPPORTED` as a top-level compatibility code beside nested stable
`GENESIS_VALIDATION_ERROR`; this regression test has not yet run against a
rebuilt addon. The Go tests also remain unrun locally because the Go toolchain
is unavailable on PATH. Issue #232 stays open until current-tree Rust/NAPI
validation, Go test evidence, and the documented legacy embedded boundary are
reviewed.

The current follow-up edits pass standalone `rustfmt --check` and
`git diff --check`. Node syntax checks for the d.ts freshness gate and NAPI
contract test, the Python SDK suite (8 tests, using an in-memory `requests`
stub because the dependency is not installed), and the d.ts declaration count
and refined-signature anchors were also checked statically. The added Core
relational validation test remains unrun until the serialized Cargo slot is
released. REST/Cargo tests remain pending; Go tests remain pending a Go
toolchain.
