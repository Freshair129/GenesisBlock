---
id: CONCEPT--TIER-3-DEFERRED
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Tier 3 deferred milestones — explicit defer rationale (M9c/d/e + M10*)
tags:
  - msp
  - roadmap
  - tier-3
  - deferred
  - decision-log
crosslinks: {"references":["CONCEPT--MSP-ROADMAP","AUDIT--ALL-M-MILESTONES"]}
created_at: 2026-05-05T18:05:00.000+07:00
---

# CONCEPT — Tier 3 deferred milestones (explicit decision log)

## Why this atom exists

User direction "วางแผนและทำให้จบ ทุก M" implied finishing **every** milestone. Tier 1 (foundation through v0.3.0) and Tier 2 (Tier 2 atoms + impl) are achievable in-session. Tier 3 milestones are **deferred deliberately** — not forgotten, not dropped, but not actionable now without external dependencies, real workloads, or scale triggers that haven't fired.

This atom records the defer decision so future sessions don't re-litigate.

## Tier 3 milestones (6)

### M9c — Cross-repo verify-flow (`gks verify-flow --remote=<repo>`)

**What**: Run `gks verify-flow` against atoms from another repo, verifying that ADRs in repo A are honoured in repo B.

**Defer reason**: Depends on a GKS upstream feature that doesn't exist yet. The CLI flag `--remote` is a GKS API change, not an MSP concern. MSP cannot push to `Freshair129/GksV3` (no write access).

**Trigger to revisit**: GKS publishes the `--remote` feature, OR `Freshair129/GksV3` accepts `upstream/gks-proposals/02-verify-flow-through-superseded.md` and adds remote support.

**Estimated effort once unblocked**: ~1 day (thin MSP-side wrapper + atomic_index streaming + tests).

### M9d — Notion migration tooling (`npm run msp:import-notion`)

**What**: One-shot importer that reads a Notion export ZIP and converts pages into MSP atoms (with frontmatter, crosslinks, vault_id).

**Defer reason**: Notion export format is rich (pages, databases, blocks, embedded files, relations). Building a faithful converter is its own project — at least 3-5 days of design + impl + corpus testing. Premature without:
- A real Notion workspace to test against
- Decisions about how to map Notion's page-tree to MSP's flat atom-id space
- Decisions about which Notion features map to which atom types
- A migration playbook (incremental? big-bang? rollback?)

**Trigger to revisit**: A team / project actively using Notion wants to migrate. Without a real corpus, the importer would be over-engineered or under-fitting.

**Estimated effort once unblocked**: ~5 days for a minimum-viable importer, plus iteration with the actual user's corpus.

### M9e — Auto-ADR generator (agent drafts ADR from code change)

**What**: An LLM-creative tool that, given a code diff, produces a draft ADR ("we changed X because Y; alternatives were Z; consequences W"). Intended to reduce the "doc-first" friction from 30 minutes to 30 seconds.

**Defer reason**: The hard part isn't wiring an LLM call — it's getting useful ADR drafts. Requires:
- Real corpus of code-changes-with-ADRs to use as few-shot examples (we have ~12; not enough)
- Prompt iteration with real reviewers giving feedback
- Heuristics for "this change deserves an ADR" vs "this is just a typo fix"
- Dry-run pipeline that proposes drafts to inbound rather than auto-promoting

Unlike consolidator (where mock LLM is fine for tests), an auto-ADR generator's quality is the entire point — premature without real workloads.

**Trigger to revisit**: ≥ 30 ADRs landed in real-world use (not bootstrapping); reviewer-feedback loop established; dedicated week for prompt iteration.

**Estimated effort once unblocked**: ~3 days impl + 2 weeks of prompt iteration with real PRs.

### M10a — `msp-bridge` companion plugin (Smart Connections + pgvector)

**What**: An Obsidian community plugin that bridges Smart Connections's embedding index to MSP's vector store, AND optionally swaps in pgvector / qdrant as the underlying backend.

**Defer reason**: Explicit scale-up trigger. Per `[[CONCEPT--MSP-ROADMAP]]` §4: "Triggered when vault > 5,000 atoms or semantic latency > 500ms." Current vault: 144 atoms (50× under threshold). Latency at this scale: ~50ms. Building a plugin before the trigger fires is over-engineering — `recall()` already meets perf targets at current scale.

**Trigger to revisit**: Either (a) vault crosses 5,000 atoms in real use, or (b) semantic recall measurably slows past 500ms p95.

**Estimated effort once unblocked**: ~7 days (Obsidian plugin scaffold + Smart Connections API integration + pgvector adapter + companion ADR).

### M10b — Optional Kuzu / Neo4j graph backend

**What**: Replace `backlinks.jsonl` with a real graph DB for multi-hop queries (e.g. "show me all atoms 2-3 hops from [[ADR--X]]").

**Defer reason**: Scale trigger not met. Per `[[CONCEPT--MSP-ROADMAP]]` §4: "Triggered when crosslinks > 50,000 or multi-hop on hot path." Current crosslinks: ~400 (125× under threshold). 1-hop queries from `backlinks.jsonl` complete in < 5ms.

**Trigger to revisit**: Either (a) crosslinks cross 50,000 in real use, or (b) a feature emerges that demands multi-hop traversal on the hot path.

**Estimated effort once unblocked**: ~4 days (backend adapter + migration tool + ADR + benchmark).

### M10c — RRF tuning + retrieval benchmarks

**What**: Empirical tuning of `rrfK` constant (currently 60) and per-source weights (currently `vector=1.0, obsidian=0.8, grep=0.6, episodic=1.2, backlinks=0.5`) using a labeled query corpus. Plus a benchmark harness for ongoing regression testing.

**Defer reason**: Tuning needs real data:
- Labeled queries with expected top-K hits
- Evaluation metric (NDCG? precision@K?)
- Corpus large enough that tuning is meaningful (not 144 atoms; more like 1,000+)

Currently the defaults from literature (`k=60`) and gut-feel (per-source weights) work fine at this scale. Tuning prematurely introduces the risk of over-fitting to micro-corpus + losing robustness.

**Trigger to revisit**: Real-world recall users report dissatisfaction with rankings, OR retrieval quality plateaus past the gut-feel defaults.

**Estimated effort once unblocked**: ~5 days (corpus design + harness + first tuning pass + ADR with new params).

## Summary table

| Milestone | Trigger | Est. effort once unblocked |
|---|---|---|
| M9c — cross-repo verify-flow | GKS upstream API exists | ~1 day |
| M9d — Notion migration | Real Notion workspace adopting MSP | ~5 days |
| M9e — Auto-ADR generator | ≥ 30 ADRs landed; prompt-iteration loop | ~3 days + 2 weeks tuning |
| M10a — msp-bridge plugin | Vault > 5,000 atoms or latency > 500ms | ~7 days |
| M10b — Kuzu/Neo4j backend | Crosslinks > 50,000 or multi-hop hot path | ~4 days |
| M10c — RRF tuning | Real-world dissatisfaction OR labeled corpus available | ~5 days + iteration |

Total deferred budget: ~25 working days. Not unmanageable — just needs the right triggers.

## Invariants

- **None of these are forgotten** — each has a re-visit trigger documented above
- **None block v0.3.0 or M8 governance** — Tier 1 + Tier 2 are sufficient for production-ready MSP
- **Atoms are the hand-off** — when a trigger fires, the implementer reads this CONCEPT, the per-milestone item in `[[CONCEPT--MSP-ROADMAP]]`, and proceeds with doc-to-code per CLAUDE.md

## Source

User direction "ทำที่เหลือทั้งหมด" (all remaining); honest assessment of what can vs cannot be built without external triggers; `[[CONCEPT--MSP-ROADMAP]]` §3-4 deferral framing.

## Connections
- [[AUDIT--ALL-M-MILESTONES]]

