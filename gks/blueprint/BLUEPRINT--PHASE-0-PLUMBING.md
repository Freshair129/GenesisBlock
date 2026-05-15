---
id: BLUEPRINT--PHASE-0-PLUMBING
phase: 3
type: blueprint
status: active
tier: process
source_type: axiomatic
vault_id: default
scale_level: L2
title: "BLUEPRINT — Phase 0 plumbing: thread the 4-tuple and attribute bag, no enforcement"
tags:
  - msp
  - ucf
  - blueprint
  - phase-0
crosslinks: {"references":["CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT","CONCEPT--ATTRIBUTE-BAG-MODEL","ADR--BRING-YOUR-OWN-ATTRIBUTES","FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK"]}
linked_symbols:
  - {"file":"packages/msp/src/policy/types.ts"}
created_at: 2026-05-14T22:21:52.048+07:00
---

# BLUEPRINT — Phase 0: plumbing

> Implementation plan for spec §11 Phase 0. **No enforcement** — this phase only threads the request shape and the attribute bag through MSP so later phases have something to consult. Behaviourally invisible to end users.

## Geography

New module:

- `packages/msp/src/policy/types.ts` — `Subject`, `Resource`, `Action`, `RequestContext`, `Decision`, `Obligation`, `AttributeBag`, plus `makeSubject` / `makeResource` constructors.

Touched (signature additions, all with backward-compatible defaults):

- `packages/msp/src/cognitive/index.ts` — `recall`, `remember`, `runTask` accept an optional `{ subject?, action?, context? }`.
- `packages/msp/src/memory.ts` — thread the tuple into the GKS recall/retain calls (as opaque pass-through metadata for now).
- `packages/msp/src/mcp/tools/*.ts` — each tool handler constructs a default `Subject` (`kind: 'mcp-client'`) and the appropriate `Action`.
- `packages/msp/src/index.ts` — Express routes construct a default `Subject` (`kind: 'user'`, anonymous) + `RequestContext` from the request.
- Atom frontmatter — `attributes:` becomes a recognised optional key; the validator already ignores unknown keys, so no validator change (per `ADR--BRING-YOUR-OWN-ATTRIBUTES`).

Not touched: GKS. `Namespace` is unchanged (per `ADR--BRING-YOUR-OWN-ATTRIBUTES`).

## Acceptance

- `tsc --noEmit` passes across all workspaces.
- Every existing test passes unchanged — Phase 0 is behaviourally invisible.
- Every MSP entry point (HTTP routes, MCP tools, facade methods) logs the full 4-tuple at debug level on every call. A new test asserts the log line shape.
- An atom carrying an `attributes:` frontmatter block validates clean and round-trips through re-index → recall with the bag intact.
- No `Decision` is consulted anywhere — `grep` for `evaluatePolicy` returns only the type definition, no call sites.

## Dependencies

- `CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT` — defines the 4-tuple types this phase creates.
- `CONCEPT--ATTRIBUTE-BAG-MODEL` — defines `AttributeBag` and the frontmatter convention.
- `ADR--BRING-YOUR-OWN-ATTRIBUTES` — confirms attributes live in atom metadata, GKS `Namespace` untouched.
- No dependency on the PDP — that is Phase 1.

## Tasks

1. **T0.1** — Create `packages/msp/src/policy/types.ts` with all six types + `AttributeBag` + constructors. Unit-test the constructors fill defaults and reject malformed shapes.
2. **T0.2** — Thread `{ subject?, action?, context? }` through the facade (`recall` / `remember` / `runTask`). Defaults: `subject = anonymous user`, `action` inferred from method, `context` from ambient (time, trace id).
3. **T0.3** — Thread the tuple through `memory.ts` into GKS calls as opaque pass-through. GKS does not interpret it.
4. **T0.4** — MCP tool handlers: each constructs `Subject{ kind: 'mcp-client' }` + its `Action`. Add the per-tool `Action` mapping table.
5. **T0.5** — Express routes: middleware constructs `Subject` + `RequestContext` from each request; attach to `req`.
6. **T0.6** — Recognise `attributes:` in atom frontmatter; ensure re-indexer + recall carry it through. Add a round-trip test.
7. **T0.7** — Add a debug-level "4-tuple" log line at every entry point; add a test asserting its shape.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §11 Phase 0.
- `CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT`, `CONCEPT--ATTRIBUTE-BAG-MODEL` — the concepts plumbed.
- `ADR--BRING-YOUR-OWN-ATTRIBUTES` — storage decision honoured here.
