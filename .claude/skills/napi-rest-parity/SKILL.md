---
name: napi-rest-parity
description: Check that a new engine capability is wired into BOTH front-ends — the NAPI method in src/lib.rs and the Axum REST route in src/router.rs — since CLAUDE.md warns they drift. Use when adding or changing an engine operation, exposing a Storage method, reviewing whether the Node addon and REST server are in sync, or when the user says "wire this into both", "check NAPI/REST parity", "is this exposed over REST too".
---

# NAPI ↔ REST Parity

GenesisBlockDB is **one core, two front-ends**: the same `Arc<Storage>` is wrapped
as NAPI `async` methods in `src/lib.rs` and as Axum handlers under `/v1/*` in
`src/router.rs`. CLAUDE.md warns: *"When you add an engine capability, you usually
wire it in both places — they can drift, so check both."* This skill makes that
check explicit so a capability does not silently ship on only one surface.

## 1. Decide whether the capability belongs on both surfaces

Not every method is meant to be on both — that is a deliberate choice, not an
oversight, and must be stated:
- **Both** (default for a user-facing engine op): add the NAPI method AND the REST
  route + handler.
- **NAPI-only** (intentional): e.g. `execute_batch` exists in the core but is
  deliberately **not** a REST route (per CLAUDE.md). If you choose NAPI-only, say so
  and why.
- **Internal** (neither): a `Storage` helper not meant for any transport.

## 2. List both surfaces and diff them

Run the parity lister to see REST routes and NAPI methods side by side:

```bash
bash .claude/skills/napi-rest-parity/scripts/parity-list.sh
```

It prints every `/v1/*` route from `src/router.rs` and every `#[napi]` method from
`src/lib.rs`. Eyeball the new capability: is it present on each surface you intended
in step 1? (The counts will not match 1:1 — status/version/swarm/consensus routes
and NAPI-only methods make the sets legitimately asymmetric. Look at the *specific*
capability, not the totals.)

## 3. Verify the contract matches on both sides

Drift is not only "missing on one side" — it is also **mismatched contracts**:
- **Body shape.** `/v1/query/hql` takes a **raw JSON string** body, but the
  Python/Go SDKs send `{ "query": "..." }`. Confirm the actual REST contract before
  changing either side (CLAUDE.md gotcha). Check `tests/rest_api_tests.rs`.
- **Semantics.** `/v1/query` is a raw scan that ignores bitemporal filtering, unlike
  the HQL path — make sure the route you add has the semantics you expect.
- **Bitemporal flags.** `as_of` / `include_invalid` / edge `retract` behavior should
  behave the same way the NAPI method does, or the difference must be intentional.

## 4. Add tests on the REST side

If you added or changed a route, add/extend a case in `tests/rest_api_tests.rs`
(the REST surface's test file). The NAPI surface is covered by `__test__/*.mjs`.
A capability wired into both surfaces needs coverage on both.

## 5. Validate

The `parity-list.sh` script is a read-only lister, not a pass/fail gate — parity is
a judgement call (intentional asymmetry exists). Record in your summary which
surfaces the capability landed on and which asymmetries are intentional.

## Notes

- Source of truth for routes: `src/router.rs` (extracted from main for testability).
  Source of truth for NAPI methods: `#[napi]`-annotated methods in `src/lib.rs`.
- Keep `index.js` / `index.d.ts` in sync with new NAPI methods (committed,
  hand-maintained until `napi build` regenerates them — see `.gitignore` note).
