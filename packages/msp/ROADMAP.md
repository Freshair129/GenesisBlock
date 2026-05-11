# MSP Roadmap

> **Source of truth**: [`gks/concept/CONCEPT--MSP-ROADMAP.md`](./gks/concept/CONCEPT--MSP-ROADMAP.md). For close-out audits: [`AUDIT--ALL-M-MILESTONES`](./gks/audit/AUDIT--ALL-M-MILESTONES.md) (v0.3.0) + [`AUDIT--V0-4-0`](./gks/audit/AUDIT--V0-4-0.md) (v0.4.0) + [`AUDIT--ARCH-DOC-CLEANUP`](./gks/audit/AUDIT--ARCH-DOC-CLEANUP.md) (Phase A, 2026-05-09) + [`AUDIT--PHASE-B-IMPL-COMPLETE`](./gks/audit/AUDIT--PHASE-B-IMPL-COMPLETE.md) (Phase B impl) + [`AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS`](./gks/audit/AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS.md) (Phase C).

## Status — post-Phase-D (architecture re-base, 2026-05-10)

After Phase A+B+C+D (PRs #65/#67/#66/#68) MSP is now explicitly **agent-agnostic** (`CONCEPT--AGENT-AGNOSTIC`) and ships a global-vs-workspace storage split (`ADR--GLOBAL-VS-WORKSPACE`).

| Phase | What | PR | Status |
|---|---|---|---|
| A | SSOT cleanup — removed `CORE_FRAMEWORK_MASTER_SPEC.md` (EVA spec) + `msp_infra_startup_architecture.md` + `SPEC--ARCHITECTURE-V2.md`; cherry-picked 3 ideas → CONCEPT atoms | #65 | ✅ merged |
| B (atoms) | `ADR--GLOBAL-VS-WORKSPACE` + `BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION` | #65 | ✅ merged |
| B (impl) | `~/.msp/` global root + `src/projects/` registry/resolve + `src/identity/migrate.ts` + 3 new MCP tools (16→19) | #67 | ✅ merged |
| C | `CONCEPT--AGENT-INTEGRATION-PATTERNS` + `docs/AGENT-INTEGRATION.md` (6-client wiring guide) | #66 | ✅ merged |
| D | `upstream/gks-proposals/06-msp-relationship-update.md` (drafted; upstream relay pending) | #68 | ✅ merged |

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

Mechanical governance via PROTO loader on top of the v0.3.0 passport core:

- **Generic PROTO loader** — any `gks/proto/PROTO--*.md` atom + linked TS predicate runs as part of `msp:validate --all`
- **9 PROTOs** shipped at `status: draft` (predicates run, observe, no fail-exit yet): SAMPLE-RULE, PHASE-GATES, SCALING-LEVEL-GATE, ALGO-PARAM-COUPLING, AUTHORITY-ENFORCEMENT, VALID-UNTIL, SUMMARY-MIN, ADR-MONOTONIC, EVIDENCE-FOR-DECISIONS
- **Cutover machinery** — when a PROTO is promoted to `stable`, `severity: error` violations fail-exit CI

## MCP tool surface (19 tools, post-Phase-B)

**Gatekeeper / candidates**: `msp_validate`, `msp_candidate`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild`

**Passport (M7f)**: `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set`

**Symbol graph**: `msp_symbol_lookup`, `msp_symbol_neighbors`, `msp_symbol_impact`, `msp_symbol_community`, `msp_symbol_search`

**Projects (Phase B)**: `msp_project_list`, `msp_project_register`, `msp_project_resolve`

**Killer demo**: `msp_recall("how did we decide rate limiting?")` returns ranked + provenance hits from GKS vector + Obsidian text + episodic + backlinks merged via RRF.

## Post-v0.4.0 — contradiction detection

Layer 0 (human rule) of `BLUEPRINT--CONTRADICTION-DETECTION-IMPL` shipped: `CLAUDE.md` § "Atom contradiction policy" + `.github/pull_request_template.md` checklist. Reviewers verify supersession discipline at PR-author time before any new mechanical layer (`PROTO--RECIPROCAL-SUPERSESSION`, `PROTO--DOMAIN-UNIQUENESS`, embedding hint, opt-in LLM judge) lands. See `ADR--CONTRADICTION-DETECTION-STACK` for the full 5-layer plan.

## Post-v0.4.0 — inbound → candidates migration (DONE)

Per `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION` (4-phase plan, all done as of 2026-05-09):

- **Phase 1 (additive)**: `msp_candidate` MCP tool + `CandidateWriter` + Knowledge Browser tab — ✅ merged
- **Phase 2 (deprecate)**: `msp_propose` marked `[deprecated]`, delegated to `CandidateWriter` — ✅ merged
- **Phase 3 (delete)**: removed `msp_propose`, `scripts/msp/propose.mjs`, `inbound/` dir — ✅ merged (commit `7eff62b`)
- **Phase 4 (atom supersession)**: `CONCEPT--INBOUND-QUEUE` superseded — ✅ merged

## Post-v0.4.0 — architecture re-base (Phases A–D, DONE)

See top of file for the table; full context in `AUDIT--ARCH-DOC-CLEANUP`. Net effect: 7 root architecture docs → 2 (`msp_spec.md`, `ROADMAP.md`); MSP declared agent-agnostic; storage split into `~/.msp/` global vs workspace per-project; MCP tool count 16 → 19.

## Counts (current, post-Phase-D)

- **~202 atoms** in `gks/` (+43 from v0.4.0)
- **663 passing tests** (+128) on Node 20 + 22
- **19 MCP tools** in `msp-mcp-server`
- **6 CLI bins**: `msp-validate`, `msp-backlinks`, `msp-run-task`, `msp-master`, `msp-mcp-server`, `msp-graph`
- **6 upstream proposals** for `Freshair129/GksV3` (5 filed, 1 drafted awaiting relay)

## Configuration to start using MSP

For full per-client wiring (Claude Code, Gemini CLI, Antigravity, Cursor, Codex, custom TS/Python), see [`docs/AGENT-INTEGRATION.md`](./docs/AGENT-INTEGRATION.md). Quick start:

```jsonc
// MCP client config (Claude Code / Cursor / Cline / Gemini CLI / Antigravity)
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_HOME": "~/.msp",                       // global identity / preferences / projects registry
        "MSP_PROJECT": "evaAI",                     // workspace project namespace
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<your-key>",
        "MSP_LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

Optional plugins: Obsidian + Local REST API + Smart Connections (configured to use `nomic-embed-text-v1.5` to match GKS 3.6.0's `createNomicEmbedder`).

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
