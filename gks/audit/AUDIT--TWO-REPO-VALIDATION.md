---
id: AUDIT--TWO-REPO-VALIDATION
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Two-repo validation — MSP atomic claims vs published @freshair129/gks
tags: &a1
  - msp
  - gks
  - audit
  - validation
  - version-control
  - upstream
crosslinks: &a2
  references:
    - ADR--EMBEDDING-MODEL-PARITY
    - ADR--MSP-OBSIDIAN-INTEGRATION
    - ADR--GRAPH-IS-GKS-DOMAIN
    - CONCEPT--EMBEDDING-STRATEGY
    - AUDIT--M7-PREP-FOLLOWUP
linked_symbols: &a3 []
created_at: 2026-05-04T12:55:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--TWO-REPO-VALIDATION
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Two-repo validation — MSP atomic claims vs published @freshair129/gks
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-04T12:55:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--TWO-REPO-VALIDATION
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Two-repo validation — MSP atomic claims vs published @freshair129/gks
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-04T12:55:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--TWO-REPO-VALIDATION
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: Two-repo validation — MSP atomic claims vs published @freshair129/gks
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-04T12:55:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — two-repo validation

## Scope

Cross-repo validation pass: do the claims in MSP's atomic notes (FRAME / CONCEPT / ADR / spec) match what `@freshair129/gks` actually ships on npm? This audit was triggered by user request "validate ทั้ง 2 repo ให้ตรงกัน" during M7-prep follow-up cleanup, and discovered a version-pinning gap that needed correction before merging PR #9 + PR #12.

## Method

| Source | What was checked |
|---|---|
| `npm view @freshair129/gks dist-tags` | npm registry latest version |
| `node_modules/@freshair129/gks/dist/src/memory/index.d.ts` | actual exported API in installed version |
| `node_modules/@freshair129/gks/dist/src/memory/vector/embedder.d.ts` | embedder providers in installed version |
| `/tmp/GksV3/CHANGELOG.md` | unreleased 3.6.0 development checkout (read-only) |
| `/tmp/GksV3/package.json` | unreleased 3.6.0 source version |
| `/tmp/GksV3/.env.example` | env var conventions in 3.6.0 source |

## Findings

### F1 — Version skew

| Surface | Version |
|---|---|
| npm registry latest | **3.5.6** |
| MSP `package.json` pin | `^3.5.6` (semver allows 3.6.x once published) |
| MSP `node_modules` installed | 3.5.6 |
| GksV3 git HEAD | 3.6.0 (unpublished) |

### F2 — API claims accuracy (PR #9 atoms)

| Claim | Status in 3.5.6 (published) | Status in 3.6.0 (source) | Verdict |
|---|---|---|---|
| `createRestObsidianAdapter` exported | ✅ exported | ✅ | **accurate** |
| `OBSIDIAN_URL` is canonical env | ❌ no env binding (caller passes `baseUrl`) | ✅ in `.env.example` | **aspirational (3.6.0+)** |
| `createNomicEmbedder` exported | ❌ providers: `'ollama' \| 'openai' \| 'mock'` only | ✅ added per CHANGELOG | **aspirational (3.6.0+)** |
| `nomic-embed-text-v1.5` is canonical | ❌ not available | ✅ default in `createNomicEmbedder()` | **aspirational (3.6.0+)** |
| GKS owns atomic graph traversal | ✅ per `SCOPE.md` | ✅ same | **accurate** |
| `gks validate --links` exists | ✅ | ✅ | **accurate** |

### F3 — Impact on shipped code (M7a / PR #12)

`src/obsidian/rest.ts` imports `createRestObsidianAdapter` from `@freshair129/gks/memory` — **resolves correctly** in 3.5.6. M7a's tests (252/252) all pass. No runtime breakage.

The aspirational claims in PR #9 atoms only affect documentation accuracy, not runtime. M7c (retrieval orchestration, future) is the first work that will hit the embedder gap.

## Corrections applied (this PR)

1. **`[[ADR--EMBEDDING-MODEL-PARITY]]`**: prepended a Status note explaining 3.6.0 is unreleased; added a "Fallback while unpublished" section showing how MSP works on 3.5.6 today (Ollama BGE-M3 / mock). The architectural decision (lock to nomic) stays valid as the **target** state.

2. **New upstream proposal**: `upstream/gks-proposals/05-publish-3.6.0.md` — asks GKS maintainer to publish 3.6.0 to npm. Blocks removing the Status note above.

3. **MSP version bump**: `package.json` 0.1.0 → 0.2.0 reflecting M7-prep follow-up + M7a milestones complete.

## NOT corrected (intentional)

- **`[[ADR--MSP-OBSIDIAN-INTEGRATION]]`** mentions `OBSIDIAN_URL` as the canonical env name — kept as-is. The 3.5.6 adapter takes `baseUrl` constructor arg, but MSP's `src/obsidian/env.ts` reads `OBSIDIAN_URL` from env independently. This is MSP's own convention layered on top; it works regardless of GKS env conventions. Forward-compatible with 3.6.0's `.env.example`.

- **GKS upstream proposals 01–04** not retrofitted with version notes — they target features that don't exist in either 3.5.6 OR 3.6.0 source (phase: 6 acceptance, verify-flow flag, backlinks API, SC parity docs). They're future work either way.

## Decision rule going forward

Adding new MSP atoms that reference GKS APIs:

1. Check `node_modules/@freshair129/gks/dist/src/memory/index.d.ts` for the symbol.
2. If absent: mark the atom with **Status note** ("aspirational pending GKS X.Y") and add an upstream proposal.
3. Never assume CHANGELOG-documented features in unreleased GKS source are usable.

This rule could be a future MSP validator check (`gks-symbol-resolves` rule) — added to M8 backlog candidates.

## Verification

```
npm view @freshair129/gks dist-tags    →  { latest: '3.5.6' }
npm test                                →  233/233 pass (PR #9 doc-only, no functional change)
npm run msp:check-links                 →  OK (now 100 atoms)
npm run msp:index                       →  100 atoms indexed
```

## Counts

- Atoms in `gks/`: 99 → 100 (this audit)
- Upstream proposals: 4 → 5 (+ `05-publish-3.6.0`)
- ADRs corrected: 1 (EMBEDDING-MODEL-PARITY Status note + Fallback section)
- Code changes: 0 (impl unaffected)

## Follow-up tasks

| Task | When | Owner |
|---|---|---|
| Submit proposal 05 to GKS maintainer along with 01–04 (see `upstream/gks-proposals/SUBMISSION.md`) | post-merge | human |
| Once GKS 3.6.0 publishes → bump installed version, remove Status note from [[ADR--EMBEDDING-MODEL-PARITY]], remove fallback section | post-publish | next milestone |
| Consider adding `gks-symbol-resolves` validator rule | M8 | M8 backlog |

## Source

User direction "validate ทั้ง 2 repo ให้ตรงกัน + ทำ version control" during PR #9 + PR #12 merge prep. Cross-checked installed `@freshair129/gks@3.5.6` against `/tmp/GksV3/` (3.6.0 dev checkout) on 2026-05-04.

## Connections

- [[ADR--GRAPH-IS-GKS-DOMAIN]]
- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[AUDIT--M7-PREP-FOLLOWUP]]
