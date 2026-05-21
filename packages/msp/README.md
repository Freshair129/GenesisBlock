# MSP — Memory & Soul Passport

> **Agent-agnostic Memory OS** that travels with any cognitive-layer client (Claude Code, Gemini CLI, Antigravity, Cursor, EVA, Hermes, openclaw, custom MCP agents). Sits on top of [`@freshair129/gks`](../gks/) for atomic-markdown storage + vector / graph / Obsidian backends.
>
> **Note:** As of 2026-05-11 monorepo migration, the canonical source for `@freshair129/gks` lives at [`packages/gks/`](../gks/) in this monorepo. The previous standalone repo `Freshair129/GksV3` is archived (read-only) on GitHub.
>
> **Atom prefix taxonomy (v2.3, 2026-05-13)**: this README uses the v2.3 vocabulary — `FRAMEWORK--` (governance / architecture, formerly `FRAME--`), `FRAME--` (Block Manifest; contract: `SPEC--GENESIS-BLOCK-MANIFEST`), `GUARD--` (was `GUARDRAIL--`), plus `STACK--`, `SPEC--`, `COGNITIVE--`, `SAFETY--`, `MOD--`. Full table: [`docs/gks/KNOWLEDGE-TYPES.md`](../gks/docs/KNOWLEDGE-TYPES.md). "Genesis Block" appears with two meanings — the **Genesis Graph Backend** (DB at `packages/gks/src/memory/graph/genesis-graph.ts`) is distinct from a **Genesis Block** (composite knowledge unit declared by a `FRAME--` manifest).

```
COGNITIVE LAYER  EVA / Hermes / openclaw / Claude Code / Gemini CLI / Antigravity / Cursor
        │ uses (agent-agnostic API — see CONCEPT--AGENT-AGNOSTIC)
        │ one-line entry: createCognitiveLayer({ root }) — see FEAT--COGNITIVE-LAYER-FACADE
        ▼
MEMORY OS        MSP (this repo) — passport: sessions / episodic / identity / retrieval / candidates
        │ uses
        ▼
KNOWLEDGE BASE   GKS (@freshair129/gks) — atomic / vector / episodic / obsidian / graph
                                          GraphBackend: in-memory / Pg / Genesis Block (TS Phase 0)
```

## What this repo is

MSP is a **passport-orchestrator** that travels with the agent, carrying:

- **Sessions** — per-turn JSONL logs (workspace)
- **Episodic memory** — importance-scored summaries (workspace)
- **Identity / soul** — profile, voice, preferences (global at `~/.msp/identity.json` + sparse per-project override)
- **Retrieval orchestration** — RRF fusion over GKS vector + Obsidian text + episodic + backlinks
- **Context compression** — token-budget-aware summarisation
- **Candidates pipeline** — `msp_candidate` MCP tool → workspace candidates → human PR → `gks/<type>/`
- **Validator** — atom shape, anti-hallucination, shift-left wikilink check (delegates link-resolution to GKS)
- **Symbol graph** — TypeScript impact analysis on `src/`

Agent-agnostic: every cognitive-layer client points to the same `msp-mcp-server` bin and gets a passport. See [`docs/AGENT-INTEGRATION.md`](./docs/AGENT-INTEGRATION.md) for per-client wiring (6 clients covered).

## Quick start (60 seconds)

For programmatic use — one import, one factory call, everything wired:

```ts
import { createCognitiveLayer } from 'msp'

const layer = await createCognitiveLayer({
  root: process.cwd(),
  slm: { tier: 'T1' },              // T1 = local Ollama + qwen2.5-coder (default)
})

await layer.remember('Cortex handles planning.', { tags: ['cortex'] })
const hits = await layer.recall('how does cortex plan?')
await layer.runTask('./.brain/tasks/FEAT--X/T1.task.yaml', { scale: 'L2' })

// Stand up the 20-tool MCP surface for EVA / Claude Code / Hermes / openclaw:
const server = layer.mcpServer()
```

The facade reuses GKS (storage) + MSP (passport + codegen) — no new MCP tools are added. It enforces seven `FRAMEWORK_MASTER_SPEC.md` invariants out of the box: §5.1 atomic short-circuit · §7.5 Memory-for-Audit stamping · §7.7.2 scale-level gate · §9.6 AUTO-GENERATED marker · §13 hybrid 4-layer retrieval · §14.1 SSOT hierarchy · §17.3 T1/T2/T3 tier routing.

Pre-reqs (optional but recommended for real codegen, not tests):

```sh
ollama pull qwen2.5-coder:7b              # T1 default
# or, on ≥16GB VRAM:
ollama pull qwen2.5-coder:14b && export OLLAMA_MODEL=qwen2.5-coder:14b
```

End-to-end demo: `npm run cognitive:quickstart` (runs `examples/cognitive-layer-quickstart.ts`).

## Authoritative docs

| Doc | Role |
|---|---|
| [`gks/framework/FRAMEWORK--MSP-ARCHITECTURE-V2.md`](./gks/framework/FRAMEWORK--MSP-ARCHITECTURE-V2.md) | Architecture SSOT (3-layer ecosystem, storage layout) |
| [`msp_spec.md`](./msp_spec.md) v2.0.3 | Full technical spec |
| [`gks/concept/CONCEPT--TAXONOMY-V2-3.md`](./gks/concept/CONCEPT--TAXONOMY-V2-3.md) | Atomic-knowledge prefix taxonomy (v2.3) |
| [`gks/spec/SPEC--GENESIS-BLOCK-MANIFEST.md`](./gks/spec/SPEC--GENESIS-BLOCK-MANIFEST.md) | Frontmatter contract for `FRAME--` Block Manifests |
| [`gks/concept/CONCEPT--AGENT-AGNOSTIC.md`](./gks/concept/CONCEPT--AGENT-AGNOSTIC.md) | MSP/agent boundary contract |
| [`docs/AGENT-INTEGRATION.md`](./docs/AGENT-INTEGRATION.md) | Per-client wiring snippets |
| [`ROADMAP.md`](./ROADMAP.md) | Milestone status + phase history |

## Layout

```
msp/
├── msp_spec.md                       full technical spec (v2.0.3)
├── ROADMAP.md                        milestone status
├── docs/
│   └── AGENT-INTEGRATION.md          per-client wiring guide
├── gks/                              canonical atom tree (committed)
│   ├── 00_index/atomic_index.jsonl
│   ├── frame/        FRAME--       (v2.3: Block Manifest atoms)
│   ├── framework/    FRAMEWORK--   (v2.3: governance / arch, was FRAME--)
│   ├── concept/  adr/  feat/  blueprint/
│   ├── spec/         SPEC--        (v2.3: data contracts)
│   ├── mod/          MOD--         (v2.3: module manifests)
│   ├── protocol/     PROTOCOL--    (interaction contracts; A2A/MCP)
│   └── audit/  proto/  task/  master/
├── src/
│   ├── lib/                          msp-home (global root resolver)
│   ├── identity/                     global + per-project identity
│   ├── projects/                     registry + .mspconfig resolution
│   ├── memory/                       sessions, episodic, candidates, backlinks
│   ├── orchestrator/                 consolidator, retrieval, compressor
│   ├── validator/                    atom shape + PROTO predicates
│   ├── symbols/                      TS symbol graph + impact analysis
│   ├── obsidian/                     REST adapter wrapper
│   ├── codegen/                      microtask runner
│   ├── mcp/                          msp-mcp-server (20 tools)
│   └── index.ts                      Knowledge Browser backend (Express)
├── ~/.msp/                           global state (per ADR--GLOBAL-VS-WORKSPACE)
│   ├── identity.json
│   ├── preferences.json
│   ├── projects.yaml
│   └── audit/<date>.jsonl
├── .brain/msp/projects/<ns>/         workspace state (gitignored)
│   ├── candidates/                   candidate atoms awaiting human PR
│   ├── sessions/  memory/  vector/  audit/
│   └── identity.override.json        sparse per-project override
├── upstream/gks-proposals/           drafts for Freshair129/GksV3
└── web/                              Knowledge Browser frontend (React + Vite)
```

## MCP server (20 tools)

```jsonc
// Claude Code: ~/.claude/mcp.json or .claude/settings.json
// Gemini CLI: ~/.gemini/config.json
// Antigravity / Cursor / Codex: equivalent MCP config
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_HOME": "~/.msp",
        "MSP_PROJECT": "evaAI",
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<your-key>"
      }
    }
  }
}
```

| Group | Tools |
|---|---|
| Gatekeeper / candidates | `msp_validate`, `msp_candidate`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild` |
| Passport | `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set` |
| Symbol graph | `msp_symbol_lookup`, `msp_symbol_neighbors`, `msp_symbol_impact`, `msp_symbol_community`, `msp_symbol_search`, `msp_symbol_trace` |
| Projects | `msp_project_list`, `msp_project_register`, `msp_project_resolve` |

Run alongside `gks-mcp-server` — clients merge tool surfaces.

## CLI bins (6)

After `npm run build`:

```sh
npx msp-validate --all                    # whole-tree atom validator
npx msp-backlinks --check                 # CI drift assertion
npx msp-run-task <T*.task.yaml>           # codegen runner
npx msp-master compose                    # 3-tier knowledge model loader
npx msp-graph                             # symbol-graph CLI
npx msp-mcp-server                        # MCP stdio server
```

## Atom workflow scripts (dev ergonomics)

Helper scripts to eliminate recurring frontmatter bugs (wrong timezone, wrong enum, reciprocal-link mistakes):

```sh
# Get a correctly-offset (+07:00 ICT) timestamp for `created_at`
npm run msp:atom-date
# → 2026-05-12T22:05:01.171+07:00

# Or UTC absolute (Z form)
npm run msp:atom-date -- --utc

# Scaffold a new atom — generates valid frontmatter + body skeleton
npm run msp:scaffold-atom -- --type=concept --slug=NEW-FEATURE
# → creates gks/concept/CONCEPT--NEW-FEATURE.md (validates clean)

# Supersede an existing atom with one or more replacements (atomic reciprocal update)
npm run msp:supersede -- --old=FEAT--FOO --new=CONCEPT--FOO,ADR--FOO,ALGO--FOO,PROTO--FOO
```

See [`AUDIT--ATOM-WORKFLOW-SCRIPTS`](./gks/audit/AUDIT--ATOM-WORKFLOW-SCRIPTS.md) for the bugs these fix and test coverage.

## Workflow (doc-to-code)

The P0..P6 chain below is the **Block Assembly** (bottom-up) half of the **Genesis Block Cycle**; its top-down counterpart is **Block Decomposition** (12-Stage Symbol Graph DAG). See `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md` for the unified vocabulary.

```
P0 FRAMEWORK → P1 CONCEPT → P2 ADR/FEAT/SPEC → P3 BLUEPRINT → (P4 TASK) → P5 src/ → P6 AUDIT
```

(`P0 FRAMEWORK--` was `P0 FRAME--` pre-v2.3; renamed per `ADR--TAXONOMY-V2-3-MIGRATION`. `FRAME--` now denotes a **Block Manifest** authored at P0 / alongside its member atoms.)

```sh
# Runtime atom proposals: use msp_candidate MCP tool — writes to
# .brain/msp/projects/<ns>/candidates/. Promotion to gks/<type>/ is a
# human PR action — see ADR--AGENT-WRITE-BOUNDARIES.
npm run msp:verify FEAT--MSP-VALIDATOR    # gate before src/
npm run msp:validate                      # MSP's own validator
npm run msp:check-links                   # crosslink resolution
npm run msp:index                         # rebuild atomic_index.jsonl
```

## Pre-commit hook

```sh
bash examples/hooks/install.sh
```

`git commit` blocks if any staged `.md` under `gks/` fails the validator. Skip with `git commit --no-verify`. Full docs: [`examples/hooks/README.md`](./examples/hooks/README.md).

## Knowledge Browser (web UI)

A visual interface for exploring the GKS atom graph: interactive 2D graph (Cytoscape.js), semantic recall, multi-vault Brain Switcher, atom inspector with frontmatter, and live counts for hotfixes / candidates queue.

### Prerequisites

- Node.js 20+
- GKS atoms in a folder structure (e.g., `gks/concept/*.md`)

### Install & run

```bash
npm install
cd web && npm install && cd ..
npm run dev          # backend (Express) + frontend (Vite) together
```

Open the app at **<http://localhost:3000>**.

### Brain Switcher

1. Click **+ Add Brain** in the top bar.
2. Provide a **Name** (e.g., "Personal Notes").
3. Provide an **Absolute Path** to the folder (e.g., `C:/Users/Name/Documents/Brain`).
4. Click **Save**.

Your brain list is persisted in `brains-config.json` (gitignored) and survives restarts.

### Tech stack

Frontend: React + Vite + TypeScript + Cytoscape.js. Backend: Node + Express + tsx. Styling: vanilla CSS, modern dark theme.

## Testing

```sh
npm test          # vitest run (~663 tests across ~80 files)
npm run typecheck # tsc --noEmit
```

CI runs both Node 20 and Node 22 on every PR.

## License

MIT
