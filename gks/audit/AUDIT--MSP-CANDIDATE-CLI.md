---
id: AUDIT--MSP-CANDIDATE-CLI
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp-candidate CLI — non-MCP agent path to MSP candidates queue
tags: &a1
  - msp
  - audit
  - cli
  - candidates
  - gemini
  - qwen
crosslinks: &a2
  references:
    - ADR--MSP-CANDIDATE-CLI
    - ADR--AGENT-WRITE-BOUNDARIES
linked_symbols: &a3
  - file: packages/msp/src/memory/candidates/cli.ts
  - file: packages/msp/package.json
  - file: scripts/msp/chmod-bins.mjs
  - file: scripts/create_atom.cjs
  - file: atom-creator/SKILL.md
  - file: atom-creator/references/taxonomy.md
  - file: gks/feat/FEAT--ATOM-CREATOR-SKILL.md
  - file: gks/blueprint/BLUEPRINT--ATOM-CREATOR-SKILL.md
  - file: gks/adr/ADR--MSP-CANDIDATE-CLI.md
created_at: 2026-05-17T03:00:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MSP-CANDIDATE-CLI
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: msp-candidate CLI — non-MCP agent path to MSP candidates queue
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-17T03:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MSP-CANDIDATE-CLI
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: msp-candidate CLI — non-MCP agent path to MSP candidates queue
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-17T03:00:00.000+07:00
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
---

## Scope
Adds a CLI entry point (`msp-candidate`) for non-MCP agents (Gemini CLI, Qwen CLI) to propose candidate atoms via MSP. Closes the gap where non-MCP agents had no compliant write path. Authority: `[[ADR--MSP-CANDIDATE-CLI]]`.

## Changes

### CLI implementation
- New: `packages/msp/src/memory/candidates/cli.ts` — wraps `CandidateWriter` with 4 subcommands: `propose`, `list`, `read`, `delete`
- Updated: `packages/msp/package.json` — new bin entry `msp-candidate` → `./dist/memory/candidates/cli.js`
- Updated: `scripts/msp/chmod-bins.mjs` — adds `dist/memory/candidates/cli.js` to chmod target list

### Cleanup
- Deleted: `atom-creator/` directory (SKILL.md, scripts/create_atom.cjs, references/taxonomy.md) — superseded by `msp-candidate` CLI
- Updated: `gks/feat/FEAT--ATOM-CREATOR-SKILL.md` status: superseded
- Updated: `gks/blueprint/BLUEPRINT--ATOM-CREATOR-SKILL.md` status: superseded

### ADR chain
- New: `gks/adr/ADR--MSP-CANDIDATE-CLI.md` — agent interface decision

## Verification
- TypeScript typecheck passed (`npm run typecheck --workspace=packages/msp`)
- Build succeeded (`npm run build --workspace=packages/msp`)
- Smoke test: `msp-candidate propose --id=FEAT--TEST-CANDIDATE --type=FEAT --title="Test Candidate" --body="test"` → created candidate, listed, deleted successfully
- Validator: `npm run msp:validate` — new atoms pass green

## Notes
- `gks propose-inbound` is deprecated; non-MCP agents must use `msp-candidate` instead
- MCP-capable agents (Claude) continue using the `msp_candidate` MCP tool — same `CandidateWriter` backend, two interfaces
