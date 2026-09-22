---
adr_id: "ADR--GENESISDB-UEE-HQL2-STAGED-ADOPTION"
version: "0.1.0b"
date: "2026-09-22"
status: candidate
owner: "Boss (Founder / Product Authority)"
complexity: "C-3"
risk: "HIGH"
scope: "GenesisBlockDB UEE-HQL2 Blueprint adoption"
---

# ADR — UEE-HQL2 Staged Adoption and Compatibility Boundary

## Status

Candidate. This ADR is reviewable but not approved. It authorizes no source, storage, API,
migration, merge or deployment change.

## Context

The extracted UEE-HQL2 Blueprint is a coherent target package, but its 190 engine obligations
are still `not_run_engine`. Its package-level checks validate schemas, examples, grammar, a
reference helper, SQLite bootstrap shape, OpenAPI structure, links and traceability; they do not
validate the current GenesisBlockDB engine, migration, crash durability, security, device or
release behavior.

The current repository already has a WAL/projection engine, SQLite substrate, graph/vector
projections, REST/NAPI surfaces and a partial `query-ir.v1` boundary. HQL currently retains
direct dispatch for some commands. The current HQL v2 specification explicitly excludes a
general planner and EXPLAIN, while the Blueprint G4 target requires a shared binder/planner/
runtime and actual counters. The current operational-boundary documents also distinguish HQL
compatibility from the typed query contract.

## Evidence baseline

| Claim | Evidence | Truth status |
|---|---|---|
| Blueprint contains 190 obligations | Extracted `TRACEABILITY.csv`, `VALIDATION-REPORT.md` | package evidence only |
| Query IR v1 is partial | `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md`, `src/lib.rs` capability reporting | current/partial |
| HQL direct and typed paths coexist | `src/lib.rs`, `src/query/`, `tests/query_ir_tests.rs` | current |
| HQL2 planner/EXPLAIN is not current contract | `docs/SPEC--HQL-V2.md` | current constraint |
| SQLite/WAL/U2/U3 status needs reconciliation | `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md`, related tests and source | contradiction to resolve |
| External readiness is unproven | mobile/release/SDK/hosted/device gates | externally unverified |

## Decision

Adopt the Blueprint as a **versioned staged target**, not as an immediate replacement for the
current engine. The implementation spine is:

```text
truth ledger -> approved compatibility contracts -> durability/transaction proof
-> snapshot/exact oracle -> query/planner/composition -> index/runtime
-> surfaces/migration/security -> release qualification
```

No stage may advertise R1 completion until its own engine evidence and the required predecessor
gates pass.

## Compatibility and authority rules

1. Preserve `query-ir.v1`, `/v1`, existing HQL callers and current disk/WAL formats while this
   ADR is a candidate. No silent switch to `query-ir.v2`, `hql.v2`, `genesis.api.v2`, `GBF2` or
   `GBO2` is permitted.
2. Any new public contract or on-disk format requires a separate approved compatibility and
   migration decision, including old-reader behavior, shadow parity, rollback and recovery proof.
3. Signed Genesis WAL remains the mutation authority. SQLite, graph and vector structures are
   projections; no caller-owned dual write or direct projection mutation is introduced.
4. Acknowledged writes may be reported stable only after the declared projections reach the
   required frontier. Replay must be idempotent and preserve public identity.
5. Snapshot generation, temporal visibility and authorization must be pinned consistently before
   cross-domain results are called exact.

## HQL, IR and planner boundary

1. HQL1/HQL v2 compatibility remains supported while the typed executor is the primary new
   integration boundary.
2. The current REST HQL compatibility envelope is proposed to accept both the existing raw JSON
   string and `{ "query": "..." }` object form, normalize them to one internal request, and
   test equivalent semantics. Removing either form requires a versioned API decision.
3. HQL-to-IR convergence must be measured by differential fixtures; current partial lowering does
   not prove one shared pipeline.
4. HQL2 planner, EXPLAIN and actual-counter work is deferred to P8/G4 and requires a follow-up
   ADR that explicitly supersedes the current no-planner/no-EXPLAIN constraint. It must not be
   smuggled into HQL P0/P1/P2 work.
5. `match_path`, `relational_named_query`, lexical search and typed filters remain unsupported
   or planned until closed schemas and acceptance tests exist.

## Migration, snapshot and security boundary

- No GBF2/GBO2 migration is started before exact reference semantics, generation publication,
  restore, shadow comparison and rollback are executable.
- Leases, temporal selectors, ACL visibility and cache/cursor authorization are contract items,
  not optimizations that can be deferred after public exposure.
- Mobile, self-host, registry, hosted CI, device, crash/soak and security claims remain
  external/unverified until their corresponding qualification evidence exists.
- Native B-tree replacement, automatic HA and multi-writer extensions are out of scope for this
  adoption decision.

## Phase mapping and gates

| Phase | Tasks | Required outcome |
|---|---|---|
| Truth/contracts | P1-P3 | 190-row ledger, approved compatibility/authority contract, closed fixtures |
| Durability/semantics | P4-P7 | journal/transaction/snapshot proof plus exact reference oracle |
| Query/composition | P8-P10 | approved HQL2/IR boundary, legal planner and cross-domain exactness |
| Qualification | P11-P16 | lifecycle, runtime, surfaces, migration, security and release evidence |

P2 is documentation-only. P3 may begin only after this ADR is approved. P4-P16 require the
corresponding phase and task gates; every implementation task uses RED -> GREEN -> deterministic
verify -> independent review in an isolated worktree.

## Consequences

Positive:

- Current consumers and storage remain protected while the target is evaluated.
- Parallel work is possible for fixtures and surface preparation, while `src/lib.rs` and
  `src/query/*` remain single-writer conflict domains.
- Blueprint package coherence is not confused with engine readiness.

Costs:

- The full Blueprint cannot be claimed or shipped in one immediate wave.
- P2/P3 require explicit owner decisions before expensive core work.
- Existing document contradictions must be reconciled instead of hidden by implementation.

## Approval checklist

Owner approval must explicitly confirm:

- staged adoption rather than immediate whole-blueprint replacement;
- preservation of current Query IR v1, `/v1`, HQL compatibility and disk/WAL formats;
- the proposed dual REST HQL input compatibility behavior;
- WAL as single mutation authority and the migration/shadow/rollback rule;
- deferral of HQL2 planner/EXPLAIN until a separate ADR;
- P3 as the next allowed phase and no source changes before P3 approval.

## Non-goals

- This ADR does not implement HQL2, a planner, EXPLAIN, GBF2/GBO2, migration, ACL, snapshots,
  new indexes, SDK parity or release qualification.
- This ADR does not mark any Blueprint engine obligation implemented.
- This ADR does not override the existing parent/peer documents until approved and reconciled.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-22 | candidate | Proposed staged UEE-HQL2 compatibility, authority and phase-boundary decision | working-tree | ATHER |
