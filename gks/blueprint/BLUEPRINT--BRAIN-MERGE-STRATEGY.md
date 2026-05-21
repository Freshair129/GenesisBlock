---
id: BLUEPRINT--BRAIN-MERGE-STRATEGY
phase: 3
type: blueprint
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: BLUEPRINT — Brain Merge Strategy — implementation plan for two-brain resolver
scale_level: feature
tags: &a1
  - msp
  - two-brain
  - blueprint
  - resolver
crosslinks: &a2
  references:
    - CONCEPT--TWO-BRAIN-ARCHITECTURE
    - ADR--BRAIN-PATH-RESOLUTION
linked_symbols: &a3
  - file: packages/msp/src/brain/resolver.ts
created_at: 2026-05-14T02:00:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--BRAIN-MERGE-STRATEGY
  phase: 3
  type: blueprint
  status: draft
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: BLUEPRINT — Brain Merge Strategy — implementation plan for two-brain resolver
  scale_level: feature
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-14T02:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--BRAIN-MERGE-STRATEGY
    phase: 3
    type: blueprint
    status: draft
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: BLUEPRINT — Brain Merge Strategy — implementation plan for two-brain
      resolver
    scale_level: feature
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-14T02:00:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--BRAIN-MERGE-STRATEGY
      phase: 3
      type: blueprint
      status: draft
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: BLUEPRINT — Brain Merge Strategy — implementation plan for two-brain
        resolver
      scale_level: feature
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-14T02:00:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — Brain Merge Strategy

Concrete implementation plan for the resolver mandated by `[[ADR--BRAIN-PATH-RESOLUTION]]`.

## File layout

```
packages/msp/src/brain/
├── resolver.ts          # public entry — resolve(query) → BrainHit[]
├── global-vault.ts      # binds to ~/.brain/ (XDG-aware on Linux/macOS;
│                        # %USERPROFILE%\.brain\ on Windows)
├── project-vault.ts     # binds to <cwd>/gks/
├── merge.ts             # dedupe + rank when both brains have hits
└── routing-table.ts     # implements the table from ADR--BRAIN-PATH-RESOLUTION

scripts/msp/init-brain.mjs        # creates ~/.brain/ tree if missing,
                                  # migrates ~/.msp/ if present
docs/walkthroughs/WALKTHROUGH--BRAIN-INIT.md
```

## Public API

```typescript
export interface BrainQuery {
  id?: string                    // exact-id lookup
  type?: AtomType                // type-restricted scan
  vault_id?: string              // honoured for global lookups (project shadow logic)
}

export interface BrainHit {
  atom: AtomRecord
  source: 'global' | 'project'
  path: string                   // absolute path on disk
}

export async function resolve(query: BrainQuery): Promise<BrainHit[]>
```

## Resolution algorithm

```
function resolve(q):
    rules = routingTable[q.type ?? '*']
    hits = []
    for source in rules.read_order:        // 'project' | 'global'
        vault = source === 'global' ? globalVault : projectVault
        for atom in vault.scan(q):
            hits.push({atom, source, path: vault.pathFor(atom.id)})
    return merge(hits)                     // dedupe by id; project shadows global
```

## Platform binding (`global-vault.ts`)

```typescript
function globalRoot(): string {
  if (process.platform === 'win32') {
    return path.join(process.env.USERPROFILE!, '.brain')
  }
  // Linux/macOS: XDG_DATA_HOME if set, else ~/.brain
  return process.env.XDG_DATA_HOME
    ? path.join(process.env.XDG_DATA_HOME, 'brain')
    : path.join(os.homedir(), '.brain')
}
```

## Project binding (`project-vault.ts`)

```typescript
function projectRoot(cwd: string): string {
  // walk up from cwd looking for a gks/ directory at the same level as .git/
  let dir = path.resolve(cwd)
  while (dir !== path.dirname(dir)) {
    if (fs.existsSync(path.join(dir, 'gks')) && fs.existsSync(path.join(dir, '.git'))) {
      return path.join(dir, 'gks')
    }
    dir = path.dirname(dir)
  }
  throw new Error('no project gks/ found above cwd')
}
```

## Merge rules (`merge.ts`)

1. Group hits by `atom.id`.
2. If a group has both `source: 'project'` and `source: 'global'`, drop the global entry (project shadows).
3. Preserve order within each source (resolver order = atom order).

## Init script (`init-brain.mjs`)

```
Steps:
1. Compute targetRoot = globalRoot()
2. If targetRoot exists → exit 0 ("already initialised")
3. Create targetRoot + subdirs: skills/ episodic/ proto/ params/
4. Touch identity.json (empty object) + registry.yaml (empty doc)
5. If ~/.msp/ exists, move its contents into targetRoot (Linux/macOS: rename;
   Windows: copy + remove src)
6. Write a SKILL--TWO-BRAIN-INITIALISED audit atom to confirm completion
```

## Test strategy (`packages/msp/test/brain/`)

- **`resolver.test.ts`** — fixture-driven: mock `~/.brain/` and `<repo>/gks/` under tmpdir, run lookups, assert source + path per the routing table.
- **`global-vault.test.ts`** — XDG_DATA_HOME honoured; Windows path uses %USERPROFILE%.
- **`project-vault.test.ts`** — walk-up logic; throws when no gks/ exists.
- **`merge.test.ts`** — project shadows global when same id; both sources preserved when ids differ.
- **`init-brain.test.ts`** — idempotent (re-run is no-op); migrates ~/.msp/ when present.

CI gate: `npm test --workspace=packages/msp` green; resolver tests <3 seconds.

## Phasing

| Step | Deliverable |
|---|---|
| **P0** (this PR) | Phase-0 atoms: CONCEPT + ADR + BLUEPRINT (= this file). No code. |
| **P1** | `routing-table.ts` + `merge.ts` (pure functions) + tests. |
| **P2** | `global-vault.ts` + `project-vault.ts` + their tests. |
| **P3** | `resolver.ts` + end-to-end resolver test against tmpdir fixtures. |
| **P4** | `scripts/msp/init-brain.mjs` + walkthrough doc + migration test. |
| **P5** | Wire resolver into MCP tools + dispatcher (Stream C P4 prerequisite). |

P5 depends on Stream C (Agent Dispatcher) reaching P4. P1–P4 are independent.

## Open questions (NOT for this PR)

- Should the global brain support multiple users on a shared machine? (Multi-tenant via `~/.brain/<user>/` — defer until use case appears.)
- Encrypted-at-rest for global? Threat-model ADR first.
- Cross-machine sync via Tailscale / Syncthing / Git annex — out of scope; recommend users handle externally.

## Connections

- [[CONCEPT--TWO-BRAIN-ARCHITECTURE]]
