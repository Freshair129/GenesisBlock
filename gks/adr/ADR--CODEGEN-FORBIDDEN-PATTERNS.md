---
id: ADR--CODEGEN-FORBIDDEN-PATTERNS
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen forbidden imports + patterns — what SLM output may not contain
tags: &a1
  - msp
  - codegen
  - forbidden-patterns
  - slm
crosslinks: &a2
  references:
    - CONCEPT--CODEGEN-MICROTASK-CONTRACT
created_at: 2026-05-03T14:08:42.151+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--CODEGEN-FORBIDDEN-PATTERNS
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Codegen forbidden imports + patterns — what SLM output may not contain
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:08:42.151+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--CODEGEN-FORBIDDEN-PATTERNS
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Codegen forbidden imports + patterns — what SLM output may not contain
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:08:42.151+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--CODEGEN-FORBIDDEN-PATTERNS
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Codegen forbidden imports + patterns — what SLM output may not contain
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-03T14:08:42.151+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — codegen forbidden imports + patterns

## Context

After post-processing, SLM output still contains predictable mistakes:

- Imports of packages we don't have installed (`zod`, `lodash`)
- Imports of Node built-ins we don't allow at the route layer (`fs`, `child_process`)
- Pages-Router idioms (`req.body`) when we use App Router (`await req.json()`)
- Debug noise (`console.log`)
- Punts (`// TODO`, `// FIXME`)

Each is mechanically detectable by regex / package-list lookup. We need to declare the rules once and apply them to every SLM completion.

## Decision

### Forbidden imports — conditional

If the imported module is **not in `package.json`**, reject:

`joi, zod, yup, ajv, uuid, lodash, ramda, axios, moment, underscore, bluebird, request`

These are the most common "default" imports SLMs hallucinate. Adding any to deps would let it through (intentional — sometimes we *do* want zod).

### Forbidden imports — absolute

Reject in all cases regardless of `package.json`:

| Module | Reason |
|---|---|
| `fs` | route handlers must not touch disk |
| `child_process` | no shell exec at the route layer |
| `net` | no raw sockets |
| `http` | use the framework's request handling |
| `"../"` (relative parent paths) | use `@/` alias for clarity + refactor safety |

### Forbidden patterns

| Pattern | Severity | Why |
|---|---|---|
| `export default` | error | Next.js App Router uses named exports |
| `req.body` | error | App Router uses `await req.json()` |
| `req\.tenantId` | error | tenantId comes from `withAuth` context, not `req` |
| `console\.(log\|debug\|info\|error\|warn)` | warning | route handlers should use Sentry / Pino, not stdout |
| `process\.env\.` | warning | route-level env access is a smell — use `@/lib/config` |
| `// TODO\|// FIXME\|// XXX` | error | SLM may not punt — retry instead |

### Required patterns per slot

| Slot | Must contain |
|---|---|
| `exports` | `export const POST =` (or named export per HTTP verb) |
| `handler` | `export async function` or `export const = async` |
| `helpers` | first non-comment line is `export function` or `export const` |

## Consequences

**Positive**

- One regex per pattern; cheap to evaluate.
- Patterns are documented per-rule with rationale, not folded into a monolithic linter.
- Severity tiers let warning patterns surface in CI without blocking the SLM loop.

**Negative**

- Pattern list will drift as the codebase evolves (e.g. when we adopt a new framework). Ownership: M3 plan loads them from `codegen_microtask_contract.yaml` so changes don't need a code release.
- Regex matching can have false positives in legitimate string contents (`"export default"` inside a comment). Acceptable — comments are stripped in post-processing.

## What this ADR does NOT decide

- The retry policy when a pattern is detected → see `[[ADR--CODEGEN-RETRY-POLICY]]`.
- The post-processing pipeline that runs first → see `[[ADR--CODEGEN-POST-PROCESSING]]`.

## Source

`msp_spec.md` §5.2 (Forbidden Imports), §5.3 (Forbidden Patterns), §5.4 (Required Patterns).

## Connections

- [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]
