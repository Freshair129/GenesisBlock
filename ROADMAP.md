# MSP Roadmap

> **Source of truth**: [`gks/concept/CONCEPT--MSP-ROADMAP.md`](./gks/concept/CONCEPT--MSP-ROADMAP.md) (atom in the canonical knowledge graph). This file is a public-facing summary; for the full plan including atom counts, dependencies, and revision policy, read the atom.

## Status (latest: PR #8 / M7-prep)

| Milestone | What | Status |
|---|---|---|
| M0–M2 | Bootstrap + atom slicing + validator | ✅ merged |
| M3 a-d | Pre-commit hook + runtime contract loader + memory writers + codegen runner + phase-6 wrapper | ✅ merged |
| M4 a-c | Bin entries + GitHub Actions CI + Ollama SLM + vitest acceptance | ✅ merged |
| M5 a-f | Pre-push hook + hotfix wrapper + 3 anti-hallucination rules + required-fields contract + ADR--HUMAN-REVIEW-GATES + shellcheck CI | ✅ merged |
| M6 | `msp-mcp-server` (6 tools over stdio MCP) | ✅ merged |
| **M7-prep** | **Architecture v2 + spec 2.0.0** (passport over Obsidian-backed GKS) | 🟢 PR #8 open, CI green |

## Coming up

### M7 — passport core
Make MSP actually carry memory + soul + retrieval (not just gatekeep writes).
- M7a — Obsidian REST client + Smart Connections probe
- M7b — Consolidator (importance + summarise)
- M7c — Retrieval orchestration (`msp_recall` fuses Obsidian + episodic + backlinks)
- M7d — Context compression (token-budget aware)
- M7e — Identity / soul layer
- M7f — MCP tool surface expansion

### M8 — governance protocol layer
Turn `FRAME--*` from descriptive docs into executable contracts via `PROTO--` atoms.
- M8a — Establish PROTO pattern
- M8b — `PROTO--PHASE-GATES`
- M8c — `PROTO--SCALING-LEVEL-GATE`
- M8d — `PROTO--ALGO-PARAM-COUPLING`
- M8e — `PROTO--AUTHORITY-ENFORCEMENT`
- M8f — Audit existing rules → promote to PROTOs

### M9 — lifecycle + distribution
Address what scale exposes.
- Decision atrophy guards (`valid_until` enforcement)
- Delegation policy
- Cross-repo verify-flow
- Migration tooling (Notion → GKS)
- Auto-ADR generator
- Windows lock parity

### M10 — scale-up backends
Only when Smart Connections plateaus.
- pgvector / qdrant via Smart Connections companion plugin
- Optional Kuzu / Neo4j graph backend
- RRF tuning + retrieval benchmarks

## Out of scope (tracked elsewhere)

- Sessions/episodic auto-hookup → agent harness (Claude Code, Cursor, EVA)
- GKS upstream `phase: 6` patch → GksV3 maintainers
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

This file mirrors a **temporary atom** with `valid_until: 2026-08-01`. Quarterly review; milestone PRs may update via `update_atomic`. Major scope changes require `supersede`.
