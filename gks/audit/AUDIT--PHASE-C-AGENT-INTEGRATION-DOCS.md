---
id: AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Phase C — agent integration patterns + wiring snippets shipped
tags:
  - msp
  - audit
  - agent-agnostic
  - integration
  - docs
crosslinks: {"references":["CONCEPT--AGENT-INTEGRATION-PATTERNS","CONCEPT--AGENT-AGNOSTIC","CONCEPT--NAMED-PROJECT-REGISTRY","ADR--GLOBAL-VS-WORKSPACE","AUDIT--ARCH-DOC-CLEANUP"]}
linked_symbols: []
created_at: 2026-05-10T07:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Phase C agent integration docs (2026-05-10)

## Scope

`[[AUDIT--ARCH-DOC-CLEANUP]]` (Phase A, 2026-05-09) declared MSP agent-agnostic
and cut the SSOT to two docs. `[[ADR--GLOBAL-VS-WORKSPACE]]` (Phase B, 2026-05-09)
specified the global-root vs workspace storage split that the agent-agnostic
claim implies. **Phase C** closes the loop by documenting the integration
contract every cognitive-layer client honors, plus concrete wiring snippets
for six clients.

The intent: prove the agent-agnostic claim is operational, not aspirational.
A user moving from Claude Code to Gemini CLI should copy 3–5 lines of config,
not relearn MSP.

## Actions taken

### Added

| File | Type | Role |
|---|---|---|
| `gks/concept/[[CONCEPT--AGENT-INTEGRATION-PATTERNS]].md` | concept atom (phase 1, stable) | Documents the 4-rule contract and the three integration shapes (MCP-native / MCP-bridged / shell-wrapped). |
| `docs/AGENT-INTEGRATION.md` | user-facing doc (not an atom) | Copy-paste wiring snippets for 6 clients (Claude Code, Gemini CLI, Antigravity, Cursor, Codex/ChatGPT, custom TS/Python agents) plus verification, switching, and caveats. |
| `gks/audit/[[AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS]].md` | audit atom (phase 6, stable) | This file. |

### Not changed

- `src/` untouched — Phase C is pure documentation.
- `msp_spec.md` untouched — already references the SSOT atoms post-Phase A.
- Existing atoms untouched — Phase C only adds.

## Coverage check against the contract

The contract from `[[CONCEPT--AGENT-INTEGRATION-PATTERNS]]`:

1. Launch `msp-mcp-server` as MCP stdio — covered in every snippet.
2. Honor project-resolution chain — covered via `MSP_PROJECT` env in every
   snippet plus `.mspconfig` documented separately.
3. Merge global identity + workspace override — `~/.msp/identity.json` and
   `MSP_HOME` documented; verification step calls `msp_identity_get`.
4. Pass `MSP_HOME` through unchanged — explicit override snippet in Claude
   Code section + general note in caveats.

## Coverage check against the three shapes

| Shape | Clients covered |
|---|---|
| MCP-native | Claude Code, Gemini CLI, Antigravity, Cursor, Codex/ChatGPT |
| MCP-bridged | Custom TS agent (`@modelcontextprotocol/sdk`), custom Python agent (`mcp` SDK) |
| Shell-wrapped | Mentioned in CONCEPT (CI bots, headless agents); not given a dedicated snippet — uses existing CLI bins (`msp-validate`, `msp-backlinks`, `msp-graph`) which are already documented per-bin. |

Six concrete wiring snippets total, satisfying the "at least 3 cognitive-layer
clients" requirement.

## What is intentionally NOT in scope

Per `[[CONCEPT--AGENT-AGNOSTIC]]` ("What MSP does not own"):

- Slash-command UX per client.
- `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` process IDs (EVA artifacts).
- Multi-agent orchestration patterns.
- Model selection / agent type taxonomy.

These are explicitly listed in `docs/AGENT-INTEGRATION.md` under the
"What MSP does NOT provide" section, with pointer back to
`[[CONCEPT--AGENT-AGNOSTIC]]`.

## Verification

- `npm run msp:index` — atomic_index.jsonl rebuilt.
- `npx tsx src/validator/cli.ts --all` — all atoms pass.
- `npm run msp:check-links` — all crosslinks resolve.
- 2 PROTO failures (PHASE-GATES, SCALING-LEVEL-GATE) remain pre-existing,
  unrelated to Phase C.

## Counts

| Before | After | Delta |
|---|---|---|
| 1 stable agent-agnostic concept (`[[CONCEPT--AGENT-AGNOSTIC]]`) | 2 (added `[[CONCEPT--AGENT-INTEGRATION-PATTERNS]]`) | +1 concept |
| 0 user-facing integration docs | 1 (`docs/AGENT-INTEGRATION.md`) | +1 doc |
| 0 Phase C audit atoms | 1 (this) | +1 audit |

## Source

Phase C of the architecture-doc cleanup (2026-05-10). Depends on Phase A+B
landing first (PR #65). Drafted as a single-developer slice — no Phase 4
(TASK) handoff per `[[ADR--PROMOTION-LEVELS]]`.

## Connections
- [[CONCEPT--NAMED-PROJECT-REGISTRY]]

