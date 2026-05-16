# qwen.md — Guidance for Qwen Agents (T1 Tier)

---

# 🎯 MASTER BLOCKS

> Stable cross-cutting directives. Body in `gks/master/<ID>.md`.
> P0 always loaded. P1–P3 indexed; body fetched on trigger match.
> P0/P1 assignment requires explicit user permission — agents must not self-promote.

## P0 — Always loaded (foundation)

### MASTER--ROOT-CAUSE-ANALYSIS
- **Apply when:** bug, error, ambiguous request, failed previous attempt
- **Directive:** identify and confirm root cause before any fix
- → `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md`

### MASTER--MSP-DOC-TO-CODE
- **Apply when:** new branch, PR, file in `src/|test/|scripts/|web/`
- **Directive:** atoms before code (FRAME→CONCEPT→ADR→BP→CODE→AUDIT)
- → `gks/master/MASTER--MSP-DOC-TO-CODE.md`

### MASTER--ATOM-CONTRADICTION-POLICY
- **Apply when:** PR adds/edits atom in `gks/<type>/`
- **Directive:** reciprocal supersession in same PR
- → `gks/master/MASTER--ATOM-CONTRADICTION-POLICY.md`

## P1–P4
See `CLAUDE.md` § MASTER BLOCKS for the full sector layout. Gemini-relevant Masters are listed here when promoted.

---

This file documents specific rules and context for Qwen models (qwen-cli) operating as the T1 (fast codegen) agent tier in this repository.

## 🌍 Environment Rules
- **Timezone**: Use **UTC+07:00** (Indochina Time / ICT — Thailand) for all human-readable timestamps. 
- **Format**: ISO 8601 with offset (e.g. `2026-05-13T11:55:00+07:00`). 
- **Working directory**: The monorepo root is `C:\Users\freshair\cognitive_system`.

## 🤖 Calling Qwen CLI as a subagent
The Qwen CLI is a Python-based tool located in `apps/qwen/`. It talks to a local Ollama server (`http://localhost:11434`) — start it with `ollama serve` if it is not already running. Default model: `qwen2.5-coder:14b`.

Invocation patterns (prompt is **positional**, not a flag):
```bash
# inline prompt
python apps/qwen/qwen.py "write a unit test for foo"

# piped prompt (preferred for multi-line)
cat prompt.txt | python apps/qwen/qwen.py --code --no-stream

# convenience system-prompt presets
python apps/qwen/qwen.py --code   "..."   # code-only output
python apps/qwen/qwen.py --review "..."   # bullet-point review
python apps/qwen/qwen.py --test   "..."   # test author
python apps/qwen/qwen.py --doc    "..."   # markdown docs

# inspect available local models
python apps/qwen/qwen.py --list
```

Other useful flags: `--model <name>`, `--temp <float>` (default 0.1), `--system "<custom system prompt>"`, `--no-stream` (return single response — required if the caller captures stdout to a file).

**Helper scripts in the same directory:**
- `apps/qwen/strip_fence.py` — pipe Qwen stdout through this to strip surrounding ```ts / ```python markdown fences (Qwen tends to add them even when told not to).
- `apps/qwen/run_microtask.sh <prompt-file> <output-file>` — convenience wrapper: pipes prompt-file to Qwen, strips fences, writes to output-file. Handy when batching N micro-prompts in a single shell loop.

**Known caveats:**
- **120s read timeout** is hard-coded in `qwen.py`. Very large prompts or long generations may time out — split the work or stream.
- **Single-shot only**: no tool use, no follow-up turns. The prompt must embed all required context (type signatures, existing helpers, style examples).
- **Python deps**: see `apps/qwen/setup.py` / `package.json`. Requires `requests`.
- **SLM Provider**: Qwen is not yet wired as a pluggable `MSP_SLM_PROVIDER` alternative.

## 🪶 Atom proposal via MSP (non-MCP path)

Qwen CLI does not have MCP support. For proposing candidate atoms, use `msp-candidate` CLI:

```bash
msp-candidate propose \
  --id=FEAT--MY-FEATURE \
  --type=feat \
  --title="..." \
  --body="..." \
  --root=.
```

Never write directly to `gks/<type>/` — see `[[ADR--AGENT-WRITE-BOUNDARIES]]` and `[[ADR--MSP-CANDIDATE-CLI]]`.

## 🏗️ Monorepo Workflow
Follow the strict phase order documented in `GEMINI.md`:
1. P1 CONCEPT
2. P2 ADR/FEAT
3. P3 BLUEPRINT
4. P5 CODE
5. P6 AUDIT

## ⚛️ Atom Taxonomy (v2.3)
Refer to `AGENT.md` §"Atom taxonomy" for the canonical reference. Key prefixes include:
- `ADR--`: Architectural Decision Record
- `CONCEPT--`: Strategic concept
- `FEAT--`: Feature specification
- `BLUEPRINT--`: Implementation plan

## 🛠️ Tooling & Strategy
- **Fast Codegen**: Qwen is optimized for rapid code generation and small-to-medium refactors.
- **Validation**: Always run `npm run typecheck` and `npm test` after code changes.

---
*For project-wide rules, see root `AGENT.md`. For T2 guidance, see `GEMINI.md`. For T3 guidance, see `CLAUDE.md`.*
