---
id: BLUEPRINT--DECISION-ATROPHY-GUARDS
phase: 3
type: blueprint
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: BLUEPRINT — Decision Atrophy Guards Implementation Plan
tags: [msp, lifecycle, atrophy, plan, m9a]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--DECISION-ATROPHY-GUARDS
    - PROTO--VALID-UNTIL
linked_symbols:
  - file: packages/msp/src/validator/proto/valid-until.ts
created_at: 2026-05-18T11:30:00+07:00
---

# BLUEPRINT — Decision Atrophy Guards

## 1. Goal

Implement the technical machinery to detect, report, and warn about atrophied (expired) GKS atoms, ensuring the repository's source of truth remains current.

## 2. Implementation Steps

### T1: Shared Expiry Logic (`packages/msp/src/validator/utils/atrophy.ts`)

- Extract and formalise the logic currently residing in `valid-until.ts` PROTO into a reusable utility.
- Function: `getAtrophyStatus(atom: AtomicIndexEntry, now: Date): AtrophyResult`
- `AtrophyResult` enum: `HEALTHY`, `NEAR_EXPIRY` (within 14 days), `EXPIRED`.
- Include helper to calculate `daysSinceExpiry` and `daysUntilExpiry`.

### T2: CLI Implementation (`packages/msp/src/validator/atrophy-cli.ts`)

- Implement a new CLI entry point using `node:util.parseArgs`.
- Command: `msp-atrophy report [--root <path>] [--json]`
- Workflow:
    1. Load Atomic Index using existing `AtomicIndex` loader.
    2. Iterate through all atoms with a `valid_until` field.
    3. Run each through the Atrophy Engine (T1).
    4. Collect results.

### T3: Reporting and Formatting

- **Text Format (Default):** Use a formatted table (e.g., using `console.table` or a simple string formatter) showing:
  - `ID`
  - `Status` (🔴 EXPIRED / 🟡 WARN)
  - `Days` (Overdue or Remaining)
  - `Title`
- **JSON Format:** Output the raw array of results for automation.

### T4: Protocol Refinement (`PROTO--VALID-UNTIL`)

- Update `packages/msp/src/validator/proto/valid-until.ts` to consume the shared engine from T1.
- Ensure consistent severity levels:
  - `EXPIRED` -> `warning`
  - `NEAR_EXPIRY` -> `info`

### T5: Package Integration

- Add `msp-atrophy` to `packages/msp/package.json` under `bin`.
- Add `npm run msp:atrophy` script.

## 3. Verification Plan

### 3.1 Unit Tests

- Create `packages/msp/test/validator/utils/atrophy.test.ts`.
- Test cases:
  - Atom with no `valid_until` (Healthy).
  - Atom with future date > 14 days (Healthy).
  - Atom with future date < 14 days (Near Expiry).
  - Atom with past date (Expired).
  - Handling of `status: superseded` (should be skipped).

### 3.2 Manual Acceptance

- Create a test atom with an expired `valid_until`.
- Run `npm run msp:atrophy` and verify it appears in the report.
- Run `npm run msp:validate` and verify the warning is emitted.
