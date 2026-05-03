---
id: ADR--MSP-VALIDATOR
phase: 2
type: adr
status: stable
vault_id: default
title: MSP validator runs as a CLI gate over inbound + the whole gks/ tree
tags:
  - msp
  - validator
  - decision
  - cli
  - pre-commit
  - contract
crosslinks: {"references":["CONCEPT--MSP-VALIDATOR"]}
created_at: 2026-05-03T06:24:24.437Z
---

# ADR — MSP validator runs as a CLI gate

## Context

`CONCEPT--MSP-VALIDATOR` motivates the gatekeeper. This ADR records *how* it runs and *what rules* it enforces. The master spec (`msp_spec.md` §4) defines required, conditional, and forbidden fields plus six anti-hallucination rules. We need to decide:

1. Where the validator runs (library / CLI / pre-commit / CI / all)
2. Which rules are hard (reject) vs soft (warn)
3. How it interacts with GKS's existing `proposeInbound()` and `inbound promote`
4. The exit-code contract for scripting

## Decision

**Implementation**: a small TypeScript library `src/validator/` with a CLI entrypoint `src/validator/cli.ts`, exposed as `npm run msp:validate`. No runtime dependency on GKS at the validator boundary — it reads `gks/00_index/atomic_index.jsonl` and inbound `.md` files directly.

**Two modes**:

- `msp:validate <file>` — single-file check against the atomic index. Used by pre-commit hooks and as a pre-promote gate.
- `msp:validate --all` — whole-tree audit. Walks every atom in `gks/` plus every candidate in `.brain/msp/projects/evaAI/inbound/`. Used in CI.

**Hard rules (exit-1 on any violation)**:

| Rule ID | What | From spec |
|---|---|---|
| `forbidden-fields` | Frontmatter contains any of: `commit_hash`, `merge_commit`, `tenant_id`, `pr_number`, `reviewer_approved_at`, `promotion_level`, `validated_at`, `validated_by`, `msp_signature`, `hash`, `execution_count`, `last_error`, `uptime`, `latency_p50`, `adr_number_override`, `feature_id_override`, `incident_id` | §4.3 |
| `id-format` | `id` (or `proposed_id` for inbound) doesn't match `^(ADR-[0-9]{3}\|FEAT--[a-z0-9-]+\|MOD--[a-z0-9-]+\|PROTO--[a-z0-9-]+\|FLOW--[a-z0-9-]+\|CONCEPT--[a-z0-9-]+\|ADR--[a-z0-9-]+\|BLUEPRINT--[a-z0-9-]+\|TASK--[a-z0-9-]+\|FRAME--[a-z0-9-]+\|AUDIT--[a-z0-9-]+\|HOTFIX--[a-z0-9]+)$` | §4.4 |
| `id-filename-match` | `id` doesn't match the file basename | §4.4 |
| `adr-monotonic` | New ADR number ≠ max(existing ADR numbers) + 1 | §4.5 `no_invented_adr_numbers` |
| `dangling-wikilink` | Any `[[X]]` in body or any ID in `crosslinks.*` not present in atomic index | §4.5 `no_dangling_wikilinks` |
| `future-date` | `created_at` > now | §4.5 `no_future_dates` |
| `summary-min` | `summary` < 10 chars or contains `TBD`/`TODO`/`FIXME`/`lorem ipsum` | §4.4 |
| `phase-status` | Status invalid for given phase (e.g. `phase: 1, status: implemented`) | §14 failure modes |

**Soft rules (warn but exit-0)**:

| Rule ID | What | From spec |
|---|---|---|
| `cite-or-mark-inferred` | Body claims a code path/line/function but no `linked_symbols` and no `epistemic.source_type: inferred` | §4.5 |
| `epistemic-confidence-missing` | `type ∈ {adr, protocol}` and no `epistemic.confidence` | §4.2 |

**Order**: parse frontmatter → check required fields → check forbidden fields → check ID format/filename → check anti-hallucination rules → check wikilinks. Stop accumulating after first 50 errors per file (cap to keep output readable).

**Exit codes**:

| Code | Meaning |
|---|---|
| 0 | All checks passed (warnings allowed) |
| 1 | One or more hard-rule violations |
| 2 | Internal error (unreadable file, missing index, malformed YAML) |

**No automatic fixing.** The validator is read-only. Agents must re-submit corrected proposals through the inbound queue.

## Consequences

**Positive**

- Schema discipline becomes mechanical, not a doc.
- Agents (Claude/Gemini/SLM) get fast, deterministic feedback before commits.
- The validator is the contract — so the contract has executable evidence.
- Works as a library, CLI, or pre-commit hook with no environment-specific glue.

**Negative**

- ~500 LOC of TS + tests to maintain.
- The forbidden-fields list is hardcoded; updating it means a code change. Mitigated by an `.brain/msp/LLM_Contract/atomic_contract.yaml` that the validator loads at runtime in a future iteration (out of scope for M2).
- Pre-commit hook adds 50–200ms to every `git commit`. Acceptable per ADR-014.

## Alternatives considered

1. **Validator as a GKS plugin.** *Rejected.* Couples MSP rules to GKS releases (per `MSP_RELATIONSHIP.md`, the contract is one-way: MSP depends on GKS, not vice versa).

2. **JSON Schema + ajv.** *Rejected for M2.* Two of the rules (ADR-monotonic and dangling-wikilink) need cross-file context that JSON Schema can't express cleanly. Custom rule engine is simpler to reason about and unit-test. Could revisit if the rule count grows past ~15.

3. **Validator runs on every recall.** *Rejected.* Read-side enforcement is too late — bad atoms are already in the store. Write-side gating (pre-commit + pre-promote) is where the leverage is. Same rationale GKS uses in ADR-014.

4. **No CLI, just a library.** *Rejected.* Pre-commit hooks need a binary. Adding `bin/msp-validate` is one line of `package.json` config.

## What this ADR does NOT change

- GKS's `inbound promote` mechanics — unchanged.
- GKS's `atomic_index.jsonl` format — read-only consumer.
- The `gks/` directory layout — unchanged.
- Phase definitions in master spec — read-only consumer.
