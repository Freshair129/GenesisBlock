---
id: ADR--HOTFIX-ESCAPE-HATCH
phase: 2
type: adr
status: stable
vault_id: default
title: Hotfix escape hatch — 48h backfill window
tags:
  - msp
  - hotfix
  - escape-hatch
  - escalation
crosslinks: {"references":["ADR--PROMOTION-WORKFLOW","FRAME--PHASE-GOVERNANCE"]}
created_at: 2026-05-03T07:08:43.472Z
---

# ADR — hotfix escape hatch

## Context

The doc-to-code loop (P1→P6) takes time. When production is on fire, on-call engineers need to ship the fix *now* and document it *afterwards*. Without an escape hatch, two bad outcomes follow:

1. The engineer ships without writing P1–P3 atoms → SSOT degrades silently and nobody notices for weeks.
2. The engineer waits to write atoms first → outage extends.

We need a sanctioned bypass that preserves the audit trail.

## Decision

```yaml
hotfix:
  allowed: true
  requires_tag: "HOTFIX"
  backfill_deadline_hours: 48
```

### How it works

1. **Tag the commit**:
   ```sh
   git commit -m "HOTFIX: rate limiter overflow — emergency cap"
   ```
2. **Open a hotfix atom** (per GksV3 ADR-014):
   ```sh
   gks hotfix open $(git rev-parse HEAD) \
     --title="prod down: rate limiter overflow" \
     --file=src/api/rate-limit.ts \
     --reason="customer escalation"
   # → HOTFIX--<7-char-sha>  with valid_to = now + 48h
   ```
3. **Pre-commit hook** (`hotfix-gate.sh`) does NOT block during the 48h window for affected files.
4. **After 48h**, pre-commit blocks any further commit on the affected files until the backfill atoms (`CONCEPT--`, `ADR--`, `BLUEPRINT--`) exist and are `stable`, AND `gks hotfix close` has been run.
5. **Backfill atoms must declare** `crosslinks.resolves: [HOTFIX--<sha>]` so the close command knows the debt is paid.

### Why 48 hours

- 24h is too tight — engineers may be off-shift or in PagerDuty mode.
- 72h+ erodes the urgency. A 48h window forces the backfill to land within two business days.
- Empirically, this is what GksV3 settled on; aligning makes our escape hatch portable.

## Consequences

**Positive**
- On-call engineers stay productive during incidents without skipping accountability.
- Every hotfix leaves a `HOTFIX--<sha>` atom and a hard deadline.
- 48h budget is per-file — multiple unrelated hotfixes don't compound.

**Negative**
- Engineers may game the tag (mark non-emergencies as HOTFIX). Mitigated by including `--reason` in the atom and reviewing during weekly retros.
- The 48h timer is enforced *locally* (this repo's pre-commit hook). Distributed enforcement (across multiple developer machines) is an orchestrator concern per GksV3 ADR-009.

## Alternatives considered

1. **No escape hatch, doc-first always.** Rejected — engineers route around the system during incidents and the SSOT loses trust at the worst possible moment.
2. **24h or 72h window.** Considered. 48h chosen as the empirical sweet spot.
3. **Auto-close hotfix when backfill detected.** Already in `gks hotfix close`. The deadline still applies — close just acknowledges.

## Source

`msp_spec.md` §10.1 (Hotfix) + `FRAMEWORK_MASTER_SPEC.md` §6.4 (referenced).
