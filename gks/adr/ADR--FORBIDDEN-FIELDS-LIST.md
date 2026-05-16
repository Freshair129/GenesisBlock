---
id: ADR--FORBIDDEN-FIELDS-LIST
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Forbidden frontmatter fields — agents must never set these
tags:
  - msp
  - validator
  - forbidden-fields
  - anti-hallucination
crosslinks: {"references":["CONCEPT--ATOMIC-WRITE-CONTRACT"],"implements":["FEAT--MSP-VALIDATOR"]}
created_at: 2026-05-03T14:08:40.802+07:00
---

# ADR — forbidden frontmatter fields

## Context

Some frontmatter fields are *derived* — they get their value from the promote workflow, the validator itself, or runtime metrics. If an agent sets them, the value is either:

- A lie (the agent doesn't know the real `commit_hash` yet), or
- A bypass attempt (claiming `validated_by: @me`), or
- Future drift (writing `execution_count: 42` that becomes stale instantly).

The validator must reject any inbound proposal containing one of these fields.

## Decision

Maintain a single hardcoded blacklist (in `src/validator/rules/forbidden-fields.ts`) grouped by reason:

### Identity forgery (5)
- `commit_hash` — set by promote workflow from git
- `merge_commit` — set when atom enters main
- `tenant_id` — context, not authored
- `pr_number` — set by CI on merge
- `reviewer_approved_at` — set by human-review gate

### Authority fields — MSP only (5)
- `promotion_level` — derived (L0/L1/L2)
- `validated_at` — timestamp from validator
- `validated_by` — identity from validator
- `msp_signature` — cryptographic seal (M4+)
- `hash` — content hash, derived

### Runtime metrics (4)
- `execution_count`
- `last_error`
- `uptime`
- `latency_p50`

### Fabrication risk (3)
- `adr_number_override` — would let agents skip ADR-monotonic
- `feature_id_override` — same idea for FEATs
- `incident_id` — incidents are tracked separately, not embedded in atoms

**Total: 17 fields.**

The list lives **in code, not in `atomic_contract.yaml`** for M2. Loading it from YAML at runtime is a **stretch goal for M3** so that contract changes don't require a code release.

## Consequences

**Positive**
- Mechanical, fast check (set membership, O(1) per field).
- Adding a new forbidden field is a one-line code change + one test.
- Clear rationale per field (the four groupings above) makes future additions easier to evaluate.

**Negative**
- 17 fields hardcoded means contract drift between code and `msp_spec.md` §4.3 is possible. Mitigated by the M3 plan to load from YAML.
- The `incident_id` rule may collide with future incident tracking. If so, rename to `external_incident_id` or move to a separate atom.

## Alternatives considered

1. **Allow but warn.** Rejected. Even a warned `commit_hash` poisons the audit trail because future readers see the value.
2. **Strip silently on promote.** Rejected. The agent won't learn the rule; same mistake recurs.
3. **Allow with a `# msp:override` magic comment.** Rejected. Magic comments are a backdoor.

## Source

`msp_spec.md` §4.3 (Forbidden Fields) — full list reproduced above.

## Connections
- [[CONCEPT--ATOMIC-WRITE-CONTRACT]]
- [[FEAT--MSP-VALIDATOR]]

