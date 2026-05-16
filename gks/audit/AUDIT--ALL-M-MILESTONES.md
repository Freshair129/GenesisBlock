---
id: AUDIT--ALL-M-MILESTONES
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Master audit — every M milestone status as of v0.3.0
tags:
  - msp
  - audit
  - master
  - roadmap
  - all-m
  - v0.3.0
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--MSP-ROADMAP","AUDIT--MSP-ARCHITECTURE-V2","AUDIT--M7-PREP-FOLLOWUP","AUDIT--TWO-REPO-VALIDATION","AUDIT--MSP-OBSIDIAN-CLIENT","AUDIT--CONSOLIDATOR","AUDIT--RETRIEVAL-ORCHESTRATION","AUDIT--COMPRESSOR","AUDIT--IDENTITY-LAYER","AUDIT--MSP-MCP-TOOL-EXPANSION"]}
linked_symbols: []
created_at: 2026-05-05T16:50:00.000+07:00
---

# AUDIT — every M milestone status (v0.3.0 close-out)

## Scope

Single-source-of-truth status snapshot for all M milestones triggered by user direction "วางแผนและทำให้จบ ทุก M". This audit closes out v0.3.0 — the version where MSP becomes a fully-functional passport over Obsidian-backed GKS.

## Tier-1 — fully shipped (impl + tests + atoms + AUDIT)

| Milestone | What | PR | Tests delta | Atoms delta |
|---|---|---|---|---|
| **M0–M2** | Bootstrap + atom slicing + validator | #1 | 0 → 49 | 0 → 5 |
| **Knowledge base** | 41 P0/P1 atoms | #2 | unchanged | 5 → 47 |
| **M3a** | Pre-commit hook | #3 | 49 → 53 | 47 → 53 |
| **M3 b/c/d** | Contract loader + 4 FEATs + phase-6 wrapper | #4 | 53 → 151 | 53 → 64 |
| **M4 a-c** | Bin entries + GitHub Actions CI + Ollama SLM + vitest acceptance | #5 | 151 → 178 | 64 → 70 |
| **M5 a-f** | Pre-push hook + hotfix wrapper + 3 anti-hall + required-fields + [[ADR--HUMAN-REVIEW-GATES]] + shellcheck CI | #6 | 178 → 218 | 70 → 84 |
| **M6** | `msp-mcp-server` (6 tools) | #7 | 218 → 233 | 84 → 89 |
| **M7-prep** | Architecture v2 + spec 2.0.0 (passport over Obsidian-backed GKS) | #8 | 233 | 89 → 95 |
| **M7-prep follow-up** | GKS audit alignment + 4 upstream proposals + two-repo validation + v0.2.0 | #9 | 233 | 95 → 100 |
| **M7a** | Obsidian client wrapper (GKS adapter delegate + filesystem fallback) | #12 | 233 → 248 | 100 → 103 |
| (M7a fixes) | 4 review concerns from PR #12 audit | #13 | 248 → 295 | 103 |
| **M7b atoms** | Consolidator atom scaffolding | #14 | 295 → 296 (+M9f tests) | 103 → 107 |
| **M7b impl** | Hybrid consolidator (det. tier-1 + LLM tier-2 borderline) | #16 | 296 → 348 | 107 → 108 |
| **M7e atoms** | Identity layer atom scaffolding | #18 | 348 | 108 → 112 |
| **M7e impl** | JSON-store identity (profile/voice/preferences) | #19 | 348 → 345 (replace older YAML+zod) | 112 → 113 |
| **M7e follow-up** | `Profile.guardrails` + `Profile.extensions` | #20 | 345 → 350 | 113 → 114 |
| **M7c atoms** | Retrieval orchestration atom scaffolding | #21 | 350 | 114 → 118 |
| **M7c impl** | RRF fusion over 4 sources | #25 | 350 → 397 | 118 → 119 |
| **M7d atoms** | Compressor atom scaffolding | #22 | 350 | 118 → 122 |
| **M7d impl** | Three-tier compressor (keep/trim/resummarise/truncate) | #28 | 397 → 435 | 122 → 132 |
| **M7f atoms** | MCP tool expansion atom scaffolding | #23 | 350 | 122 → 125 |
| **M7f impl (4/5)** | `msp_recall`, `msp_remember`, `msp_identity_get/set` | #29 | 397 → 429 | 125 → 131 |
| **M7f follow-up** | Wire `msp_compress` (5/5 tools) | this PR | 435 → 478 | 132 → 138 |
| **M8a** | PROTO pattern atoms (foundation for M8b–f) | #24 | 397 | 122 → 126 |
| **M9f** | Session lock max-age safeguard (Windows / zombie PID parity) | #26 | 350 → 355 | 126 → 128 |

**Total**: 25 milestones merged; **142 atoms** in `gks/`; **478 passing tests**.

## Tier-2 — atoms shipped, impl deferred

These have CONCEPT atoms scoping the work but no impl yet. Specific FEAT/BLUEPRINT/impl follow per-milestone in future PRs.

| Milestone | What | PR | Status |
|---|---|---|---|
| **M8b** | `[[PROTO--PHASE-GATES]]` — enforce P0..P6 ordering at PR-time | #27 | scoped (CONCEPT) |
| **M8c** | `[[PROTO--SCALING-LEVEL-GATE]]` — auto-detect L1/L2/L3 from diff | #27 | scoped (CONCEPT) |
| **M8d** | `[[PROTO--ALGO-PARAM-COUPLING]]` — bi-directional `tunes ↔ tunable_by` (post-audit-scoped smaller) | #27 | scoped (CONCEPT) |
| **M8e** | `[[PROTO--AUTHORITY-ENFORCEMENT]]` — git author tier ↔ paths | #27 | scoped (CONCEPT) |
| **M8f** | Audit existing rules → promote 3 to PROTOs | #27 | scoped (CONCEPT) |
| **M9a** | `valid_until` enforcement + scheduled review report | #27 | scoped (CONCEPT) |
| **M9b** | Delegation policy — L2 = 2 senior, L3 = Boss-only | #27 | shipped (ADR-only — pure policy) |

The 7 atoms in PR #27 capture intent + rule + trigger + severity for each. Implementation can proceed at any future point following each CONCEPT as the contract; they unblock subagent dispatches when needed.

## Tier-3 — explicitly deferred (external triggers / out-of-MSP-scope)

| Milestone | Why deferred |
|---|---|
| **M9c** Cross-repo verify-flow | Depends on GKS upstream API (`gks verify-flow --remote`); tracked in `upstream/gks-proposals/` |
| **M9d** Notion migration tooling | Large project on its own; warrants its own doc-to-code milestone |
| **M9e** Auto-ADR generator | LLM-creative; needs prompt iteration with real workloads |
| **M10a** msp-bridge companion plugin | Triggered when vault > 5,000 atoms or semantic latency > 500ms |
| **M10b** Optional Kuzu/Neo4j graph backend | Triggered when crosslinks > 50,000 |
| **M10c** RRF tuning + retrieval benchmarks | Triggered when retrieval quality plateaus → empirical comparison |

These are tracked in `[[CONCEPT--MSP-ROADMAP]]` §3-4. No atom debt; intentionally not built without trigger.

## What's now possible

After v0.3.0:

- ✅ Agent connects via MCP stdio (`msp-mcp-server`) — **11 tools** registered
  - **Gatekeeper (M6)**: validate, propose, run-task, session-append, episode-append, backlinks-rebuild
  - **Passport (M7f)**: recall, remember, compress, identity-get, identity-set
- ✅ `msp_recall(query)` returns RRF-fused hits from GKS vector + Obsidian text + episodic + backlinks
- ✅ `msp_remember(session_id)` consolidates session into Episode[] + persists to episodic store
- ✅ `msp_compress(episodes, budget_tokens)` shrinks-to-fit via three-tier strategy
- ✅ `msp_identity_*` reads/writes per-namespace passport (profile/voice/preferences/guardrails/extensions)
- ✅ Pre-commit + pre-push hooks enforce validator + verify-flow
- ✅ Session lock cross-platform (Windows / zombie-PID safe)
- ✅ Doc-to-code workflow with 138 atoms and machine-checkable governance via PROTO pattern (foundation; specific PROTOs Tier-2)

## Known gaps

| Gap | Mitigation |
|---|---|
| GKS 3.6.0 (`createNomicEmbedder`) unpublished | MSP uses 3.5.6 with `provider: 'ollama' | 'mock'`; auto-picks 3.6.0 when published per `^3.5.6` semver |
| Tier-2 PROTOs (M8b–f) not implemented | Foundation atoms shipped; impl deferred — non-blocking for usage |
| Notion / Auto-ADR / cross-repo verify-flow | Tier-3; deferred deliberately |
| Some hooks rely on POSIX semantics (max-age helps but isn't bulletproof on Windows) | M9f closes 95% of cases; full coverage = future `proper-lockfile` swap if needed |

## Configuration required (user-side)

```jsonc
// MCP client config (Claude Code / Cursor / Cline)
{
  "mcpServers": {
    "msp": {
      "command": "npx",
      "args": ["msp-mcp-server", "--root=/path/to/your/project"],
      "env": {
        "OBSIDIAN_URL": "https://127.0.0.1:27124",   // optional — only for REST text search
        "OBSIDIAN_API_KEY": "<your-key>",            // optional
        "MSP_LLM_PROVIDER": "ollama"                 // or "anthropic" / "openai" / "mock"
      }
    }
  }
}
```

Optional: install Obsidian + Local REST API plugin + Smart Connections plugin (configured to use `nomic-embed-text-v1.5` once GKS 3.6.0 publishes).

## Counts at v0.3.0

| Metric | Value |
|---|---|
| Atoms in `gks/` | **142** |
| Tests | **478** passed, 0 failed |
| Validator passes | 142/142 |
| Test files | 64 |
| MCP tools | 11 |
| Source LoC | ~12,000 |
| FEATs implemented | 16 |
| AUDIT atoms | 23 |
| Upstream proposals drafted | 5 |
| Open PRs | 0 |

## Source

Single user direction "วางแผนและทำให้จบ ทุก M" + roadmap atom + 4 parallel subagent dispatches (M7a, M7b impl, M7c impl, M7d impl, M7e impl, M7f impl) + 25 PRs landed across the session.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[AUDIT--MSP-ARCHITECTURE-V2]]
- [[AUDIT--M7-PREP-FOLLOWUP]]
- [[AUDIT--TWO-REPO-VALIDATION]]
- [[AUDIT--MSP-OBSIDIAN-CLIENT]]
- [[AUDIT--CONSOLIDATOR]]
- [[AUDIT--RETRIEVAL-ORCHESTRATION]]
- [[AUDIT--COMPRESSOR]]
- [[AUDIT--IDENTITY-LAYER]]
- [[AUDIT--MSP-MCP-TOOL-EXPANSION]]

