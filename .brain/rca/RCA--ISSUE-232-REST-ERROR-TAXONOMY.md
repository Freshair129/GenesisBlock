---
status: active
superseded_by: null
issue: 232
---

# RCA — REST error taxonomy divergence

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

1. Add a shared REST JSON envelope and stable category/code mapping.
2. Normalize JSON, query-string, and path extractor rejections as validation
   errors while retaining their diagnostics and HTTP status.
3. Keep existing Query IR/HQL top-level codes and full messages as compatibility
   aliases; select stable categories from typed request/route context, never by
   parsing the diagnostic string.
4. Return durability only from the typed Storage recovery-required state.
5. Update Python, Go, and Studio remote clients to read nested envelope details
   while retaining their legacy fallbacks.
6. Add route-level category coverage and keep the API reference and SRS aligned.

## Remaining contract limit

REST request-shape validation, capability requests, route-specific query and
conflict failures, integrity operations, and recovery-required writes can be
covered with explicit context. The core still returns reason-only NAPI errors;
therefore the REST adapter's fallback category for other engine failures is
route-defined rather than a typed semantic classification. NAPI/embedded
callers also do not receive the REST envelope. Replacing this boundary with a
shared typed error kind is a separate core/API change. GB-SRS-API-004's full
cross-surface typed-category acceptance remains open; this patch does not claim
that acceptance is complete.

## Verification scope

- `cargo check --no-default-features --features bins --bin genesis-db-server`:
  passed.
- `cargo test --no-default-features --test rest_api_tests`: 47 passed, 0 failed,
  including the five-category route matrix.
- `cargo test --no-default-features --lib
  recovery_required_write_returns_durability_envelope`: 1 passed, 7 filtered;
  this exercises the recovery-required write route.
- `cargo build --no-default-features --features bins --bin genesis-db-server`:
  passed.
- Python client contract suite: 7 passed, including stable/legacy code fields.
- Studio transport tests: 5 passed; `tsc -b` and production `vite build` passed.
- Go client tests were added but could not be run: no Go executable is installed
  or available on PATH in this environment.

Passing these checks demonstrates the bounded REST/SDK compatibility behavior
only; it does not prove native/NAPI category parity, and the unrun Go tests
remain outstanding verification.
