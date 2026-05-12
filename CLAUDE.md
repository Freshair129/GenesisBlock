# CLAUDE.md — Monorepo guidance for Claude Code

This is the project-internal contract for how Claude Code (and human contributors driving it) should work in this monorepo.

## What this repo is

**MSP Monorepo** — collapses the GKS storage engine and the MSP orchestrator into a single workspace.

- **`packages/gks/`** — Genesis Knowledge System engine. Atomic storage, vector/graph logic, and validation.
- **`packages/msp/`** — Memory & Soul Passport orchestrator. Uses GKS to manage agent memory and identity.

> **History note.** GKS used to live in a separate repo `Freshair129/GksV3`. As of the 2026-05-11 monorepo migration (`ADR--MONOREPO-STRUCTURE`), that repo is **archived (read-only) on GitHub** and the canonical source lives here at `packages/gks/`. Do not push to `GksV3`; do not look there for current code or docs.

### Authoritative docs:
- `packages/msp/gks/frame/FRAME--MSP-ARCHITECTURE-V2.md` — Top-level architecture
- `packages/msp/msp_spec.md` — Full MSP spec
- `packages/gks/README.md` — GKS engine documentation
- `packages/msp/ROADMAP.md` — Execution plan

## Monorepo Workflow

### Boundary Rules (ADR--MONOREPO-STRUCTURE)
- **GKS** (`packages/gks/`) MUST NOT import from **MSP** (`packages/msp/`).
- **MSP** depends on **GKS** via the workspace protocol (`"@freshair129/gks": "workspace:*"`).
- GKS is still publishable as a standalone library.

### Useful Commands
Run these from the repo root:
```bash
npm run msp:index           # Regen atomic_index.jsonl in packages/msp
npm run test                # Run tests across all packages
npm run typecheck           # Run tsc --noEmit across all packages
npm run build               # Build all packages
```

Per-package commands:
```bash
npm test --workspace=packages/gks
npm run msp:validate --workspace=packages/msp
```

## Doc-to-code workflow (mandatory)

Every milestone follows this phase order:
1. **FRAME** (architecture) -> `packages/msp/gks/frame/`
2. **CONCEPT** (intent) -> `packages/msp/gks/concept/`
3. **ADR/FEAT** (decision) -> `packages/msp/gks/adr/` or `packages/msp/gks/feat/`
4. **BLUEPRINT** (plan) -> `packages/msp/gks/blueprint/`
5. **CODE** (actual src) -> `packages/msp/src/` or `packages/gks/src/`
6. **AUDIT** (what shipped) -> `packages/msp/gks/audit/`

## Atom Integrity
- Atoms must validate (`npm run msp:validate`) before commit.
- Crosslinks must resolve (`npm run msp:check-links`).
- Context tracing is mandatory: read related atoms before coding.

## Branching + PR conventions
- Branch name: `claude/msp-<milestone>-<slug>`
- Squash-merge with a 1-paragraph summary.
- CI must be green on both Node 20 + 22.

## Environment Rules
- **Timezone**: Use **UTC+07:00** (Indochina Time / ICT — Thailand) for human-readable timestamps in ISO 8601 offset format. Validator uses UTC absolute internally; `Date.parse()` handles offset correctly. Authoring rule: write `created_at: 2026-05-12T11:55:00.000+07:00` (TH wall-clock) — NOT `Z` suffix unless you've computed UTC yourself.

