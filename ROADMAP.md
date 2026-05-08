# MSP Roadmap

> **Source of truth**: [`gks/concept/CONCEPT--MSP-ROADMAP.md`](./gks/concept/CONCEPT--MSP-ROADMAP.md). For close-out audits: [`AUDIT--ALL-M-MILESTONES`](./gks/audit/AUDIT--ALL-M-MILESTONES.md) (v0.3.0) + [`AUDIT--V0-4-0`](./gks/audit/AUDIT--V0-4-0.md) (v0.4.0).

## Status — v0.4.0 (governance mechanism complete)

### Tier 1 — shipped + impl

| Milestone | What | Status |
|---|---|---|
| M0–M2 | Bootstrap + atom slicing + validator | ✅ merged |
| M3 a-d | Pre-commit hook + runtime contract loader + memory writers + codegen runner + phase-6 wrapper | ✅ merged |
| M4 a-c | Bin entries + GitHub Actions CI + Ollama SLM + vitest acceptance | ✅ merged |
| M5 a-f | Pre-push hook + hotfix wrapper + 3 anti-hallucination rules + required-fields contract + ADR--HUMAN-REVIEW-GATES + shellcheck CI | ✅ merged |
| M6 | `msp-mcp-server` (6 tools over stdio MCP) | ✅ merged |
| M7-prep | Architecture v2 + spec 2.0.0 (passport over Obsidian-backed GKS) | ✅ merged |
| M7-prep follow-up | GKS audit alignment + 5 upstream drafts + v0.2.0 | ✅ merged |
| **M7a** | Obsidian client wrapper (GKS adapter delegate + filesystem fallback) | ✅ merged |
| **M7b** | Consolidator (hybrid det. tier-1 + LLM tier-2 borderline) | ✅ merged |
| **M7c** | Retrieval orchestration (RRF fusion over 4 sources) | ✅ merged |
| **M7d** | Context compressor (three-tier: keep/trim/resummarise/truncate) | ✅ merged |
| **M7e** | Identity / soul layer (profile/voice/preferences/guardrails/extensions) | ✅ merged |
| **M7f** | MCP tool surface — 5 passport tools wrapping M7b/c/d/e (server now has 11 tools) | ✅ merged |
| **M8a** | PROTO pattern foundation (atom type + loader scaffold) | ✅ merged |
| **M9f** | Session lock max-age safeguard (Windows / zombie-PID parity) | ✅ merged |

### Tier 2 — draft PROTOs (predicates running, awaiting promotion to stable)

| Milestone | Status |
|---|---|
| M8b — `PROTO--PHASE-GATES` | 🟡 draft impl (predicate runs, no fail-exit) |
| M8c — `PROTO--SCALING-LEVEL-GATE` | 🟡 draft impl |
| M8d — `PROTO--ALGO-PARAM-COUPLING` (post-audit-scoped smaller) | 🟡 draft impl |
| M8e — `PROTO--AUTHORITY-ENFORCEMENT` | 🟡 draft impl |
| M8f — Promote 3 existing rules → PROTOs (SUMMARY-MIN, ADR-MONOTONIC, EVIDENCE-FOR-DECISIONS) | 🟡 draft impl (overlap with core) |
| M9a — `PROTO--VALID-UNTIL` (decision atrophy guards) | 🟡 draft impl |
| M9b — Delegation policy (L2 = 2 senior, L3 = Boss-only) | ✅ shipped (ADR-only — pure policy) |

### Tier 3 — explicitly deferred (external triggers)

| Milestone | Why deferred |
|---|---|
| M9c — Cross-repo verify-flow | depends on GKS upstream API |
| M9d — Notion migration | large project on its own |
| M9e — Auto-ADR generator | LLM-creative; needs prompt iteration |
| M10a — msp-bridge plugin | trigger: vault > 5,000 atoms or semantic latency > 500ms |
| M10b — Optional Kuzu/Neo4j | trigger: crosslinks > 50,000 |
| M10c — RRF tuning + benchmarks | trigger: retrieval quality plateau |

## What v0.4.0 enables (vs v0.3.0)

11 MCP tools (unchanged) — **PLUS** mechanical governance via PROTO loader. v0.3.0 had passport core; v0.4.0 adds:

- **Generic PROTO loader** — any `gks/proto/PROTO--*.md` atom + linked TS predicate runs as part of `msp:validate --all`
- **9 PROTOs** shipped at `status: draft` (predicates run, observe, no fail-exit yet): SAMPLE-RULE, PHASE-GATES, SCALING-LEVEL-GATE, ALGO-PARAM-COUPLING, AUTHORITY-ENFORCEMENT, VALID-UNTIL, SUMMARY-MIN, ADR-MONOTONIC, EVIDENCE-FOR-DECISIONS
- **Cutover machinery** — when a PROTO is promoted to `stable`, `severity: error` violations fail-exit CI

**Gatekeeper (M6)**: `msp_validate`, `msp_propose`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild`

**Passport (M7f)**: `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set`

**Killer demo**: `msp_recall("how did we decide rate limiting?")` returns ranked + provenance hits from GKS vector + Obsidian text + episodic + backlinks merged via RRF.

## Post-v0.4.0 — contradiction detection

Layer 0 (human rule) of `BLUEPRINT--CONTRADICTION-DETECTION-IMPL` shipped: `CLAUDE.md` § "Atom contradiction policy" + `.github/pull_request_template.md` checklist. Reviewers verify supersession discipline at PR-author time before any new mechanical layer (`PROTO--RECIPROCAL-SUPERSESSION`, `PROTO--DOMAIN-UNIQUENESS`, embedding hint, opt-in LLM judge) lands. See `ADR--CONTRADICTION-DETECTION-STACK` for the full 5-layer plan.

## Counts at v0.4.0

- **159 atoms** in `gks/`
- **535 passing tests**
- **71 test files**
- **27 AUDIT atoms**
- **9 PROTO atoms** (1 sample + 8 governance, all draft)
- **5 upstream proposals** drafted for `Freshair129/GksV3`

## Configuration to start using MSP

```jsonc
// MCP client config (Claude Code / Cursor / Cline)
{
  "mcpServers": {
    "msp": {
      "command": "npx",
      "args": ["msp-mcp-server", "--root=/path/to/your/project"],
      "env": {
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<your-key>",
        "MSP_LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

Optional plugins: Obsidian + Local REST API + Smart Connections (configured to use `nomic-embed-text-v1.5` once GKS 3.6.0 ships).

## Out of scope (tracked elsewhere)

- Sessions/episodic auto-hookup → agent harness (Claude Code, Cursor, EVA)
- GKS upstream `phase: 6` patch → `Freshair129/GksV3` (drafted in `upstream/gks-proposals/01`)
- GKS 3.6.0 publish → `Freshair129/GksV3` (drafted in `upstream/gks-proposals/05`)
- Cross-tenant auth → production deployment / orchestrator
- Embedder choice → per-project config

## Principles

1. Self-contained (MSP + GKS + Obsidian, no separate orchestrator)
2. Doc-to-code discipline (atoms before code, every milestone)
3. 3-tier gating (pre-commit + pre-push + CI)
4. Local-first (embeddings, hooks, validator)
5. Pluggable not opinionated (Ollama/Anthropic, Smart Connections/RAG, GKS/neo4j)
6. Versioned governance (FRAME → PROTO → FEAT → CI gate)

## Revision

This file mirrors a **temporary atom** with `valid_until: 2026-08-01`. Quarterly review; milestone PRs may update via `update_atomic`. Major scope changes require `supersede`. v0.3.0 close-out audit: [`gks/audit/AUDIT--ALL-M-MILESTONES.md`](./gks/audit/AUDIT--ALL-M-MILESTONES.md).
