# Change Request: Epistemic Memory Commit and Belief Revision Integration

## CR ID

`CR--EPISTEMIC-MEMORY-COMMIT-AND-BELIEF-REVISION`

## Owner

MSP Memory / Distiller / Governance

## Scale

L2-L3

## Summary

Integrate explorer handoff outputs and rollback branch summaries into the existing memory pipeline: session log, episodic memory, tiered memory distillation, epistemic metadata, epistemic states, and belief revision.

## Background

MSP already defines append-only memory sessions, episodic memory, tiered memory distillation, epistemic states, epistemic metadata, and belief revision. This CR formalizes how branch outputs and explorer summaries enter that pipeline.

## Scope

In scope:

* Define `MemoryCommit`
* Bind `MemoryCommit` to episodic memory
* Add epistemic metadata to distilled claims
* Promote memory through the existing tiered distillation process
* Trigger belief revision when new memory contradicts confirmed beliefs
* Prevent direct overwrite of confirmed identity-level beliefs

Out of scope:

* Runtime checkpoint implementation
* GRL coverage reporting
* HQL grammar
* KV cache implementation

## MemoryCommit Schema

```yaml
memory_commit:
  id: MEM--TAX-RULES-SUMMARY-001
  source_branch: BRANCH--TAX-EXPLORATION-001
  type: distilled_context
  summary: "Tax deduction rules require pre-tax calculation before net salary computation."
  node_refs:
    - SRS--TAX-DEDUCT-RULES
    - ADR--TAX-ROUNDING-POLICY
  decisions:
    - "Use monthly taxable income as calculation base."
  risks:
    - "Rounding policy conflicts with legacy payroll module."
  open_questions:
    - "Confirm jurisdiction-specific tax bracket source."
  epistemic:
    confidence: 0.72
    source_type: inferred
    duration: temporary
    valid_until: 2026-08-04
  validity:
    based_on_versions:
      SRS--TAX-DEDUCT-RULES: "v1.4.2"
```

## Memory Pipeline

```text
Agent branch / explorer output
→ HandoffCapsule
→ MemoryCommit
→ Episodic memory candidate
→ Tiered Memory Distillation
→ Narrative memory
→ Identity memory only if repeatedly confirmed
```

## Belief Revision Rule

If a `MemoryCommit` contradicts a confirmed belief:

1. Do not overwrite the confirmed belief directly.
2. Mark the new claim as evidence.
3. Move the old belief to `contested` if contradiction threshold is met.
4. Open a recovery window.
5. Downgrade or reformulate only after repeated contradiction.
6. Emit an audit artifact.

## Epistemic Rules

* `status` tracks document lifecycle.
* `epistemic_state` tracks confidence in the claim.
* `epistemic_state` may regress.
* `status` should remain governed by document lifecycle rules.
* Raw hidden chain-of-thought must not be stored.
* Store summaries, evidence refs, decisions, risks, and unresolved questions.

## Acceptance Criteria

1. MemoryCommit can be written from an AgentBranch or HandoffCapsule.
2. MemoryCommit enters episodic memory first, not identity memory directly.
3. Distillation follows the existing tiered memory pipeline.
4. Claims carry epistemic metadata.
5. Contradictory evidence triggers belief revision instead of direct overwrite.
6. Confirmed beliefs can move to contested state.
7. Deprecated beliefs remain for audit but are not retrieved for critical decisions.
8. Audit artifacts are produced for belief revision events.

## Related Specs

* `CONCEPT--MEMORY-SESSIONS`
* `CONCEPT--MEMORY-EPISODIC`
* `CONCEPT--TIERED-MEMORY-DISTILLATION`
* `CONCEPT--EPISTEMIC-STATES`
* `CONCEPT--EPISTEMIC-METADATA`
* `CONCEPT--BELIEF-REVISION`
