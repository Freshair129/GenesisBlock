---
id: ADR--CONTRADICTION-DETECTION-STACK
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Contradiction detection stack — 5 layers (human rule + reciprocal supersession + domain uniqueness + embedding hint + LLM judge)
tags:
  - msp
  - validator
  - contradiction
  - supersession
  - decision
  - ci
  - governance
crosslinks: {"references":["CONCEPT--ATOM-CONTRADICTION-DETECTION","ADR--ANTI-HALLUCINATION-RULES","ADR--HUMAN-REVIEW-GATES","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-08T17:32:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — contradiction detection stack

## Context

`[[CONCEPT--ATOM-CONTRADICTION-DETECTION]]` argues for a 4-layer detection stack plus a Layer-0 human rule. This ADR fixes the **decisions**: which layers to build, in what order, with which thresholds and trade-offs, and what each one's output looks like.

## Decision

### The 5 layers (Layer 0 + 4 mechanical)

```
Layer 0  Human rule        CLAUDE.md + PR template     Prevents most cases at PR-author + reviewer time
Layer 1  Reciprocal sup.   PROTO predicate             Blocking; zero false positives
Layer 2  Domain unique.    Schema field + PROTO        Blocking; zero false positives once domain is filled
Layer 3  Embedding hint    CI bot + GKS embedder       Non-blocking PR comment; probabilistic
Layer 4  LLM judge         CI bot + small LLM, opt-in  Non-blocking PR comment; probabilistic + $$
```

### Layer 0 — Human rule (CLAUDE.md + PR template)

**Decision:** ship as part of Phase 1 of the BLUEPRINT, regardless of what mechanical layers come after. This costs nothing and catches the majority of intentional drift before any code runs.

**Specification:**

`CLAUDE.md` adds a section:

```markdown
## Atom contradiction policy

If a new atom claims something that conflicts with an existing
`status: stable` atom of the same type, the conflicting atom MUST be
explicitly superseded in the same PR:

1. Add the old atom's id to the new atom's `crosslinks.supersedes`
2. Add the new atom's id to the old atom's `crosslinks.superseded_by`
3. Flip the old atom's `status` to `superseded`

The PR reviewer must verify the above before approving any PR that
adds or modifies an atom in `gks/<type>/`.
```

PR template (`.github/pull_request_template.md`) adds a checklist line.

### Layer 1 — Reciprocal supersession (blocking)

**Decision:** new PROTO `[[PROTO--RECIPROCAL-SUPERSESSION]]` modeled on `[[PROTO--ALGO-PARAM-COUPLING]]`.

**Specification:**

For every atom A in the index:

```
For each B in A.crosslinks.supersedes:
    REQUIRE: B exists in index
    REQUIRE: A.id ∈ B.crosslinks.superseded_by
    REQUIRE: B.status === 'superseded'

For each B in A.crosslinks.superseded_by:
    REQUIRE: B exists in index
    REQUIRE: A.id ∈ B.crosslinks.supersedes
    REQUIRE: A.status === 'superseded'
```

**Severity:** error (blocks PR via existing validator exit code 1).

**Why blocking:** zero false positives — it's pure crosslink integrity. If the rule fires, the data is genuinely inconsistent and must be fixed.

### Layer 2 — Domain-scoped uniqueness (blocking, after Phase 2 schema migration)

**Decision:** add a `domain:` field to ADR / FEAT / CONCEPT frontmatter. Define the initial taxonomy. New PROTO `[[PROTO--DOMAIN-UNIQUENESS]]` enforces at most one stable atom per (type, domain) pair.

**Specification:**

```yaml
# new optional frontmatter field on ADR/FEAT/CONCEPT
domain: persistence | retrieval | identity | embedding-strategy |
        crosslinks | inbound | monorepo-structure | governance |
        validator | mcp-tooling | ...
```

**Initial taxonomy:** seed with ~15 domains derived from existing atom clusters in `gks/`. Adding a domain requires updating the enum (one-line PR).

**PROTO predicate:**

```
For every (type T, domain D):
    Let S = { atom ∈ index | atom.type === T, atom.domain === D, atom.status === 'stable' }
    REQUIRE: |S| ≤ 1
```

**Severity:** error (blocking).

**Migration:** Phase 2 of the BLUEPRINT backfills `domain:` on existing ADR/FEAT/CONCEPT atoms. Atoms without `domain:` are exempt from the PROTO during a grace period (configurable in `atomic_contract.yaml`); after the grace period, missing domain is itself an error.

**Why blocking:** zero false positives once domain is set. Adding a second stable atom in a domain is a bug — either it should supersede the first, or one of them should be in a different domain.

### Layer 3 — Embedding similarity hint (non-blocking PR comment)

**Decision:** GitHub Action that runs on PRs touching `gks/<type>/*.md`. Uses GKS's existing `createEmbedder({ provider: 'auto' })` which falls back to mock in CI environments without nomic. Posts a single PR comment summarizing high-similarity matches.

**Specification:**

- For each new/changed `gks/<type>/*.md` in the diff:
  - Compute embedding of (title + body, truncated to embedder context)
  - Cosine-compare against embeddings of all `status: stable` atoms of the same `type` in the index
  - For pairs with `cos_sim > 0.85` and not already linked via `supersedes` / `superseded_by`, include in PR comment
- PR comment format:
  ```
  ## Atom similarity check (non-blocking)
  
  | New atom | Similar to | Cosine similarity |
  |---|---|---|
  | CONCEPT--FOO | [[CONCEPT--BAR]] (stable) | 0.91 |
  
  If these cover overlapping scope, consider supersession (see CLAUDE.md
  § "Atom contradiction policy"). Comment is informational; does not
  block the PR.
  ```
- If no pairs cross the threshold, post nothing (don't spam clean PRs)

**Threshold rationale:** 0.85 chosen empirically; tunable in `.github/contradiction-detection.yml`. Experience-tune after first 10 PRs of data.

**Severity:** comment only. Never blocks merge.

**Why non-blocking:** embedding similarity is a hint, not a verdict. Two atoms can be 0.92 similar and entirely complementary; two atoms can be 0.65 similar and directly contradict. False positives are tolerable when output is a comment.

### Layer 4 — LLM contradiction judge (non-blocking PR comment, opt-in)

**Decision:** opt-in via PR label `contradiction-check` or repo-level config flag. Uses a small LLM (Claude Haiku 4.5 default; configurable). Cost-budget enforced.

**Specification:**

Trigger:
- PR has the label `contradiction-check`, OR
- Repo config sets `contradiction_judge_default: true` and PR touches `gks/<type>/*.md`

For each new/changed atom:
- Identify top-K=5 most similar existing stable atoms via Layer 3's embedding output
- Send diff + those K atoms + this prompt to the LLM:
  ```
  You are reviewing a PR adding a new atom to a knowledge base.
  Read the new atom and the K most-similar existing atoms.
  For each existing atom: does the new atom contradict any specific
  claim it makes? Cite the contradicting passage from each side.
  Output JSON: { "contradictions": [{ "old_atom": ..., "claim": ...,
  "new_claim": ..., "severity": "definite" | "possible" | "none" }] }
  ```
- Post a PR comment summarizing only `definite` and `possible` findings

Cost guard:
- Hard cap: 50¢/PR; if K × atom_token_count exceeds budget, truncate K
- Soft monthly cap configurable in repo settings; over-budget skips the layer entirely with a comment

**Severity:** comment only.

**Why opt-in:** it costs real money and is probabilistic. Default-off keeps the bill predictable; opt-in flag lets reviewers escalate hard-to-judge PRs.

### Severity matrix

| Layer | Trigger | Output | Blocks merge? | False positive tolerance |
|---|---|---|---|---|
| 0 | PR review | Reviewer judgment | n/a | n/a (human) |
| 1 | `npm run msp:validate --all` (CI) | validator error | ✅ yes | 0 |
| 2 | `npm run msp:validate --all` (CI) | validator error | ✅ yes | 0 (after grace) |
| 3 | GitHub Action on PR | PR comment | ❌ no | medium (~30% acceptable) |
| 4 | Opt-in GitHub Action | PR comment | ❌ no | medium (~30% acceptable) |

## Consequences

### Positive

- **Reciprocal supersession is no longer a discipline issue** — Layer 1 mechanically enforces it
- **Adding a second authority in a domain becomes loud** — Layer 2 forces the supersession decision into the open
- **Forgotten predecessors get flagged** — Layer 3 catches "did you mean to supersede this you forgot existed"
- **Subtle body-claim contradictions get a probabilistic check** — Layer 4 catches things embedding similarity misses
- **Existing PROTO infrastructure is reused** — Layers 1 and 2 plug into the validator; minimal new infra
- **Cost stays bounded** — only Layer 4 incurs $; default-off

### Negative

- **Layer 2 needs a one-time domain backfill** — ~50 atoms in `gks/` need `domain:` added. Mechanical work.
- **Domain taxonomy becomes a thing to govern** — adding domains is one-line PRs but still requires consensus on naming. Mitigation: start with ~15 domains seeded from existing clusters; expand reactively.
- **Layer 3 adds CI time** — embedding 50–150 atoms per PR is ~2–5 sec. Negligible relative to existing test runtime.
- **Layer 4 PR comments may be wrong** — reviewer must judge. Mitigation: explicitly mark "non-blocking, advisory" in every comment; track precision over time and tune prompt.

### Neutral

- **No change to atom write path** — agents and humans write atoms the same way. Detection runs at validator-time (Layers 1, 2) or PR-time (Layers 3, 4).
- **Compatible with `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]`** — candidates layer is excluded; only `gks/<type>/` atoms are checked.

## Alternatives considered

### A. Skip the human rule (Layer 0); rely on machines

**Rejected because:** Layers 1 and 2 catch only their respective failure modes. The largest share of contradictions in practice will be "author didn't realize there was a prior atom" — a problem prevented at PR-author time more cheaply than detected after the fact. Layer 0 is free; skipping it makes the others work harder.

### B. Build only Layer 1 (reciprocal supersession); skip 2–4

**Rejected because:** Layer 1 only catches *declared* supersessions that are missing reciprocity. It does nothing for "two stable atoms in same domain, no supersession declared." Layer 2 is the workhorse against silent drift.

### C. Make Layer 3 (embedding) blocking instead of advisory

**Rejected because:** embedding similarity is probabilistic; blocking would produce false-positive merges-blocked that erode trust in CI. Comments are the right output.

### D. Use a large LLM (Claude Sonnet / GPT-4) for Layer 4 instead of Haiku

**Rejected because:** cost. Sonnet at K=5 atoms per PR can cost $0.50/PR easily. Haiku at the same workload is ~$0.05. Quality difference for "spot a contradiction" is small. Make the model configurable; default Haiku.

### E. Run Layer 4 on every PR, no opt-in

**Rejected because:** budget. Default-off keeps the monthly bill predictable; teams who want to spend can flip the flag.

### F. Detect contradictions inside `.brain/candidates/` too

**Rejected because:** candidates are agent-private (per `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]`). Validating them imports the inbound problem with new vocabulary. Detection lives at the canon boundary; candidates are explicitly outside.

## What this ADR does NOT change

- Existing structural validator rules — keep working as-is
- Existing PROTO predicates — keep running; new PROTOs are additive
- Atom write paths — no MCP tool changes
- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` candidate boundary — Layer 3/4 only scan `gks/<type>/`
- The supersession crosslink format — same `crosslinks.supersedes` / `superseded_by` as today

## Source

- `[[CONCEPT--ATOM-CONTRADICTION-DETECTION]]` — motivation and stack design
- `[[PROTO--ALGO-PARAM-COUPLING]]` — template for Layer 1 PROTO
- `[[PROTO--PHASE-GATES]]` — template for Layer 2 PROTO
- GKS `createEmbedder` (3.6.0) — Layer 3 dependency
- Anthropic Claude Haiku — Layer 4 default LLM
- `[[ADR--ANTI-HALLUCINATION-RULES]]` — complementary write-time rules

## Connections
- [[ADR--HUMAN-REVIEW-GATES]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

