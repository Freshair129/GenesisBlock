---
id: ADR--PROMOTION-LEVELS
phase: 2
type: adr
status: superseded
tier: genesis
source_type: axiomatic
vault_id: default
title: Promotion levels — L0 raw / L1 validated / L2 reviewed
tags:
  - msp
  - promotion
  - levels
  - lifecycle
  - superseded
crosslinks: {"references":["ADR--PROMOTION-WORKFLOW","CONCEPT--INBOUND-QUEUE"],"superseded_by":["ADR--AGENT-WRITE-BOUNDARIES"]}
created_at: 2026-05-03T14:08:43.030+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

> ⚠️ **Superseded by [`[[ADR--AGENT-WRITE-BOUNDARIES]]`](./[[ADR--AGENT-WRITE-BOUNDARIES]].md)** (Phase 4 of `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`, 2026-05-09). The L0/L1/L2 promotion-level model is replaced by a simple two-state model: `status: candidate` (in `.brain/.../candidates/`) → `status: stable` (in `gks/<type>/` after human PR + CI). Body preserved as historical context.

# ADR — promotion levels

## Context

`[[ADR--PROMOTION-WORKFLOW]]` defines three gates (submission, validator, human review). Each gate moves an atom through a different *level* of confidence. Without explicit level labels, downstream tooling can't tell "validator-passed but not yet human-reviewed" from "fully promoted".

## Decision

Three explicit levels, ordered:

| Level | Meaning | Where it lives | Set by |
|---|---|---|---|
| **L0** | Submitted, not validated | `.brain/msp/projects/<ns>/inbound/<ID>.rev-X.md` | agent (via `gks propose-inbound`) |
| **L1** | Validator passed, awaiting human review | same path; flagged in `gks inbound list` | validator (after green run) |
| **L2** | Human-reviewed, merged into canonical store | `gks/<type>/<ID>.md` with `status: stable` | promote workflow (after `gks inbound promote`) |

### State transitions

```
(none) ─submit─▶ L0 ─validate─▶ L1 ─review─▶ L2
                  │              │
                  │              └─reject─▶ rejected/
                  │
                  └─reject─▶ rejected/
```

L1 → L0 backwards transition happens when the validator is re-run after a fix and previously flagged errors come back. L2 is terminal in the lifecycle (further changes go through `update_atomic` proposals).

### Field encoding

`promotion_level` is **NOT** a frontmatter field on the atom — it's a derived attribute computed from:

- L0: file in `inbound/`, no `validated_at` envelope
- L1: file in `inbound/`, `validated_at` set in audit log
- L2: file in `gks/<type>/`, `status: stable`

This is why `promotion_level` appears in `[[ADR--FORBIDDEN-FIELDS-LIST]]` — agents must not write it.

## Consequences

**Positive**
- Explicit levels make tooling predictable (`gks inbound list --level=L1` shows what's awaiting review).
- Level transitions are observable in the audit log.
- Derived nature prevents tampering.

**Negative**
- Three states adds complexity vs simple "in inbound or not". Mitigated by treating L0/L1 as a single "candidate" tier in most UIs.
- Re-validation may flip L1 → L0; UIs must show this without alarm.

## Alternatives considered

1. **Two levels (raw / promoted).** Rejected — loses the validator-passed signal, which is the cheapest filter for human reviewers.
2. **Five+ levels (raw / linted / typechecked / reviewed / signed).** Considered for M4 once we add cryptographic signing. Out of scope here.
3. **Frontmatter field for `promotion_level`.** Rejected per `[[ADR--FORBIDDEN-FIELDS-LIST]]` — derived, not authored.

## Source

`msp_spec.md` §8.1 (Promotion Levels).

## Connections
- [[CONCEPT--INBOUND-QUEUE]]

