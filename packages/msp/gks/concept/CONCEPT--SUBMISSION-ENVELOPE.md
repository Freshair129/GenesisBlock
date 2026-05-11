---
id: CONCEPT--SUBMISSION-ENVELOPE
phase: 1
type: concept
status: superseded
tier: genesis
source_type: axiomatic
vault_id: default
title: Submission envelope — provenance frontmatter wrapping every inbound atom
tags:
  - msp
  - inbound
  - envelope
  - provenance
  - superseded
crosslinks: {"references":[],"superseded_by":["CONCEPT--KNOWLEDGE-LAYERS-V2"]}
created_at: 2026-05-03T07:01:50.823Z
---

> ⚠️ **Superseded by [`CONCEPT--KNOWLEDGE-LAYERS-V2`](./CONCEPT--KNOWLEDGE-LAYERS-V2.md)** (Phase 4 of `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION`, 2026-05-09). `CandidateWriter` writes a plain atom directly to `.brain/.../candidates/${proposed_id}.md` — no envelope wrapper, no `proposal_id`, no `proposal_type`. Body preserved as historical context.

# CONCEPT — submission envelope

Every proposal in the inbound queue is wrapped by an **envelope frontmatter** that records *who* submitted *what* and *why*. The envelope is read by the promote workflow and stripped before the atom lands in `gks/`.

## Shape

```yaml
---
proposal_id: "20260502-093000-claude-new_atomic-msp-techspec"
proposal_type: new_atomic     # new_atomic | update_atomic | supersede | deprecate
target_file: "gks/adrs/ADR-080.md"
submitted_by: "@claude-opus-4-7"
submitted_at: "2026-05-02T09:30:00Z"
rationale: "Document MSP technical surface for new joiners"
---
# (atomic body — the real frontmatter + content begins here)
```

## Field semantics

| Field | Required | Purpose |
|---|---|---|
| `proposal_id` | yes | unique handle for this proposal (timestamp + agent + slug) |
| `proposal_type` | yes | one of the four proposal types — see `CONCEPT--PROPOSAL-TYPES` |
| `target_file` | yes | predicted destination under `gks/` (validator checks the type prefix) |
| `submitted_by` | yes | agent handle (must match `registry.yaml`) |
| `submitted_at` | yes | ISO-8601; future-dating is rejected by the validator |
| `rationale` | yes | one-sentence motivation for the human reviewer |

## What the validator does with it

- Verifies `submitted_by` exists in `registry.yaml`
- Cross-checks `target_file` matches the inner atom's `type` (e.g. `gks/adrs/` requires `type: adr`)
- Refuses `submitted_at` in the future (per `ADR--ANTI-HALLUCINATION-RULES`)
- Rejects envelope fields appearing in the inner atom's frontmatter (those are MSP-only)

## What the promote workflow does with it

- Strips the envelope before writing to `gks/<type>/`
- Records `submitted_by` + `submitted_at` in the audit log
- Sets the canonical atom's `created_by` from envelope's `submitted_by`

## Note on the GKS published format

The current `@freshair129/gks` package uses a thinner envelope (`proposed_id`, `review_id`, `proposed_at`) without `rationale` or `target_file`. MSP wraps that with the spec-shape envelope when a higher-fidelity audit trail is required.

## Source

`msp_spec.md` §3.1 (Submission Envelope).
