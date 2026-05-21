# GKS CLI — CLAUDE.md

# Created At: 2026-05-14 14:00:00 +07:00 (v0.1.0-skeleton)

## What This Is

`@freshair129/gks-cli` — command-line interface to the Genesis Knowledge System.

**Consumers (both):**

- **Human developers** — run `gks search`, `gks get`, `gks list` in terminal
- **AI agents** — Claude/Gemini run these commands via Bash tool to query GKS without
  needing to read raw JSONL files or import TypeScript packages directly

## Status

**SKELETON** — entry points exist but all commands return `not yet implemented`.
Next step: wire `gks search` to `packages/gks` search API.

## Commands (planned)

```bash
gks search <query>          # semantic search, --type, --limit, --json
gks get <ATOM-ID>           # fetch single atom by ID
gks list [--type TYPE]      # list all atoms, filterable
gks validate                # run msp:validate
gks index                   # regenerate atomic_index.jsonl
```

## Agent Usage Pattern

When an AI agent needs to query GKS without importing TypeScript:

```bash
# In Bash tool or shell script:
gks search "episode retention" --json | jq '.results[0]'
gks get CONCEPT--TAXONOMY-V2-3 --json
gks list --type FEAT --json
```

Output is plain text by default. `--json` flag gives structured output for agent parsing.

## Stack

- TypeScript + Node.js (ESM)
- `commander` for argument parsing
- Imports `@freshair129/gks` and `@freshair129/msp` directly (same runtime)
- No server needed — reads atom store directly

## Dev

```bash
npm run dev -- search "test"   # run via tsx (no build needed)
npm run build                  # tsc → dist/
npm run typecheck
```

## Rules

- `--json` flag MUST be machine-readable (no color codes, no extra text, valid JSON)
- Plain text output MAY use color (chalk) for human readability
- Never require a running server — CLI reads atom store directly
- Exit code 0 = success, 1 = error, 2 = not found
