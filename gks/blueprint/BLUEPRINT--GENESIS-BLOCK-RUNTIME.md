---
id: BLUEPRINT--GENESIS-BLOCK-RUNTIME
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Genesis Block Runtime — implementation plan for composite execution
scale_level: feature
tags: &a1
  - msp
  - genesis-block
  - runtime
  - blueprint
  - phase-e5
crosslinks: &a2
  references:
    - CONCEPT--GENESIS-BLOCK-RUNTIME
    - SPEC--GENESIS-BLOCK-MANIFEST
    - BLUEPRINT--AGENT-DISPATCHER
    - CONCEPT--AGENT-AGNOSTIC
linked_symbols: &a3
  - file: packages/msp/src/genesis/types.ts
  - file: packages/msp/src/genesis/loader.ts
  - file: packages/msp/src/genesis/composer.ts
  - file: packages/msp/src/genesis/executor.ts
  - file: packages/msp/src/genesis/cli.ts
created_at: 2026-05-14T03:35:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--GENESIS-BLOCK-RUNTIME
  phase: 3
  type: blueprint
  status: draft
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — Genesis Block Runtime — implementation plan for composite
    execution
  scale_level: feature
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-14T03:35:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--GENESIS-BLOCK-RUNTIME
    phase: 3
    type: blueprint
    status: draft
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — Genesis Block Runtime — implementation plan for composite
      execution
    scale_level: feature
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-14T03:35:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      domain: blueprint
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# BLUEPRINT — Genesis Block Runtime

Implementation plan for `[[CONCEPT--GENESIS-BLOCK-RUNTIME]]`. Lands in Phase E5 of the agentic-monorepo pivot.

## File layout

```
packages/msp/src/genesis/
├── types.ts          # GenesisManifest, ExecuteOptions, ExecuteResult
├── loader.ts         # loadManifest() + loadMembers()
├── composer.ts       # composePrompt() — pure
├── executor.ts       # executeBlock() — orchestrates loader + composer + dispatch
└── cli.ts            # msp-genesis-exec <blockId> --prompt "<text>"

packages/msp/test/genesis/
├── loader.test.ts     # tmpdir fixture, frontmatter parsing
├── composer.test.ts   # pure section ordering, missing-dimension skip
├── executor.test.ts   # mocks dispatch(), verifies orchestration
└── cli.test.ts        # CLI help + happy path with mocked dispatch
```

## Public API

```typescript
// types.ts
export type Dimension =
  | 'algo' | 'concept' | 'cognitive' | 'runbook' | 'params'

export interface GenesisManifest {
  id: string
  members: {
    algo?: string[]
    concept?: string[]
    cognitive?: string[]
    runbook?: string[]
    params?: string[]
  }
  daci?: { driver?: string; approver?: string; contributor?: string[]; informed?: string[] }
}

export interface ExecuteOptions {
  root: string
  prompt: string
  tier?: 'T1' | 'T2' | 'T3'
}

export interface ExecuteResult {
  block_id: string
  output: string
  members_loaded: number
  tier_used: 'T1' | 'T2' | 'T3'
  duration_ms: number
}
```

## Component contracts

### loader.ts

```typescript
export async function loadManifest(
  blockId: string,
  root: string,
): Promise<GenesisManifest>
```

- Looks up `<root>/gks/genesis/GENESIS--<blockId>.md`
- Parses YAML frontmatter via `yaml` package
- Reads `members.core.*` (preferred) **or** `members.*` (flat — for tests / simpler manifests)
- Returns a normalised `GenesisManifest`
- Throws if the file is missing or frontmatter is malformed

```typescript
export interface LoadedMember {
  id: string
  dimension: Dimension
  body: string
  path: string
}

export async function loadMembers(
  manifest: GenesisManifest,
  root: string,
): Promise<Record<Dimension, LoadedMember[]>>
```

- For each dimension and each id, scans `<root>/gks/<dim>/<id>.md`
  (fallback: walk `<root>/gks/`)
- Parses each atom; extracts body (everything after the closing `---`)
- Returns a record keyed by dimension; missing files are silently skipped
- Never throws on per-member failure — log to stderr and move on

### composer.ts

```typescript
export function composePrompt(
  manifest: GenesisManifest,
  members: Record<Dimension, LoadedMember[]>,
  userPrompt: string,
): string
```

- Pure function. No IO.
- Emits sections in fixed order: Cognitive → Algorithm → Concept → Runbook → Params → User Request
- Section header: `## Context (Cognitive)`, `## Algorithm`, `## Concept`, `## Runbook`, `## Params`, `## User Request`
- Empty dimensions are omitted entirely (no empty headers)
- Multi-member dimensions concatenate bodies with a blank line between

### executor.ts

```typescript
export async function executeBlock(
  blockId: string,
  opts: ExecuteOptions,
): Promise<ExecuteResult>
```

Pseudocode:
```
1. const t0 = Date.now()
2. const manifest = await loadManifest(blockId, opts.root)
3. const members = await loadMembers(manifest, opts.root)
4. const prompt = composePrompt(manifest, members, opts.prompt)
5. const result = await dispatch({
     type: 'codegen',
     severity: 'regular',
     prompt,
     ...(opts.tier ? { budget_hint: opts.tier } : {}),
   })
6. return {
     block_id: manifest.id,
     output: result.output,
     members_loaded: Σ members[dim].length,
     tier_used: result.tier_used,
     duration_ms: Date.now() - t0,
   }
```

### cli.ts

```
msp-genesis-exec <blockId> --prompt "<text>" [--tier T1|T2|T3] [--root <dir>] [--json] [--help]
```

Exit codes:
- 0 — success
- 1 — execution failed (manifest missing, dispatch error)
- 2 — bad arguments

## Test plan

| File | Coverage |
|---|---|
| `loader.test.ts` | tmpdir with mock `[[GENESIS--FOO]].md` → assert manifest parses; missing file → throws; flat `members.*` shape supported |
| `composer.test.ts` | All 5 dimensions → 5 sections + user; only 2 dimensions → 2 sections; empty members → just user; section order is stable |
| `executor.test.ts` | Mock `dispatch()` + a fake manifest/members → assert `executeBlock` passes the composed prompt; budget_hint forwarded when `opts.tier` set; `members_loaded` correct |
| `cli.test.ts` | `--help` prints usage; missing `--prompt` → exit 2; happy path → calls executeBlock + prints output |

## Out of scope (Phase E5)

- Validator enforcement of member-id resolution (future PROTO)
- Multi-block composition / chaining
- Per-member token budget / truncation
- Caching the composed prompt
- Streaming output from `dispatch()`

## Acceptance

Phase E5 closes when:
1. The 3 atoms are authored and pass `msp-validate`
2. All 4 test files green under `vitest run`
3. `tsc --noEmit` clean (no `any`, strict mode)
4. `msp-genesis-exec --help` works end-to-end via the bin entry

## Connections
- [[SPEC--GENESIS-BLOCK-MANIFEST]]
- [[BLUEPRINT--AGENT-DISPATCHER]]
- [[CONCEPT--AGENT-AGNOSTIC]]

