---
id: CONCEPT--ATOM-CONTRADICTION-DETECTION
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Atom contradiction detection — 4-layer stack catching semantic conflict
  between new and existing knowledge
tags: &a1
  - msp
  - validator
  - contradiction
  - conflict
  - supersession
  - semantic
  - governance
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - ADR--ANTI-HALLUCINATION-RULES
    - CONCEPT--MSP-VALIDATOR
    - CONCEPT--KNOWLEDGE-LAYERS-V2
created_at: 2026-05-08T17:30:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--ATOM-CONTRADICTION-DETECTION
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Atom contradiction detection — 4-layer stack catching semantic conflict
    between new and existing knowledge
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-08T17:30:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--ATOM-CONTRADICTION-DETECTION
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Atom contradiction detection — 4-layer stack catching semantic conflict
      between new and existing knowledge
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-08T17:30:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--ATOM-CONTRADICTION-DETECTION
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Atom contradiction detection — 4-layer stack catching semantic conflict
        between new and existing knowledge
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-08T17:30:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — atom contradiction detection

## Problem

The current validator catches **structural** problems (schema, links, ID format, write-time anti-hallucination) but does not catch **semantic contradiction** between atoms. Two atoms can both pass every existing rule and still claim opposite things about the same scope.

### Worked examples (real failure modes)

```
ADR-003   (status: stable, 2026-01)  "use Postgres as the primary store"
ADR-018   (status: stable, 2026-05)  "use SQLite for all persistence"
                                      ↑ does not supersede ADR-003
                                      ↑ both pass the validator today
                                      ↑ readers/agents now have two contradicting authorities
```

```
CONCEPT--FOO body: "MSP must NOT do graph traversal — that's GKS scope"
CONCEPT--BAR body: "MSP traverses the atom graph in src/memory/graph.ts"
                    ↑ contradiction lives in markdown body
                    ↑ no validator rule reads body claims
```

```
ADR--A    (stable)  "all retrieval goes through msp_recall"
ADR--B    (stable)  "agents may call GKS recall directly for low-latency paths"
                    ↑ same domain, opposite policy
                    ↑ not marked as superseded
```

### Why this matters more than schema bugs

A schema bug fails noisily — the validator yells, the PR is red, you fix it. A semantic contradiction fails **silently**. Both atoms pass; both end up in `gks/<type>/`; both get indexed; both get returned by `msp_recall`. An agent reading the canon now has two contradictory premises and cannot tell which is authoritative. This is the worst kind of failure mode: **invisible until it bites in a downstream decision**.

### What the existing system catches vs. doesn't

| What | Caught by | Notes |
|---|---|---|
| dangling reference (`[[CONCEPT--MISSING]]`) | `dangling-wikilinks` rule | structural |
| duplicate ID, ID format wrong | `id-format`, `id-filename-match` | structural |
| ADR number collision / non-monotonic | `adr-monotonic` rule | structural |
| missing required field | `required-fields` rule | schema |
| placeholder body (TBD/TODO/FIXME) | `summary-min` rule | shift-left |
| invented version number | `no-invented-versions` rule | write-time anti-hallucination |
| claim without citation | `cite-or-mark-inferred` rule | write-time anti-hallucination |
| FEAT without ADR predecessor | PROTO `PHASE-GATES` | chain integrity |
| ALGO ↔ PARAM reciprocal coupling | PROTO `ALGO-PARAM-COUPLING` | crosslink integrity |
| `supersedes` declared but not reciprocated | **❌ NOT CAUGHT** | gap |
| two stable ADRs in same domain, no supersession | **❌ NOT CAUGHT** | gap |
| body claims that contradict another atom's body | **❌ NOT CAUGHT** | gap |
| atom that *should* have superseded an old one but didn't | **❌ NOT CAUGHT** | gap |

### What we have today: explicit `supersedes` / `superseded_by` crosslinks

These work, but they are **manual** and **trust-the-author**. Nothing today verifies:

- Reciprocity (A says it supersedes B, but B doesn't acknowledge it back)
- Completeness (the new atom *should* have superseded an old one but didn't)
- Body-level consistency (the markdown narrative inside the body matches the supersession declaration)

## Intent

A 4-layer stack that catches contradiction before it reaches stable canon. Each layer is **additive**, not replacement — they catch progressively harder cases at progressively higher cost:

```
Layer 1 — Reciprocal supersession             cheap   deterministic   100% recall on the case it covers
Layer 2 — Domain-scoped uniqueness            cheap   deterministic   needs schema migration
Layer 3 — Embedding similarity hint           medium  probabilistic   PR comment, not a gate
Layer 4 — LLM contradiction judge             $$      probabilistic   PR comment, opt-in per repo policy
```

And one **prerequisite that prevents more than any detector catches:**

> **Layer 0 — Human rule (CLAUDE.md + PR template):**
> "If your new atom conflicts with an existing stable atom, you must supersede it in the same PR. Reviewer must verify."

### Layer 1: Reciprocal supersession check

PROTO predicate. Mechanical. Catches:

- A.crosslinks.supersedes contains B → B.crosslinks.superseded_by must contain A; B.status must be `superseded`
- B.crosslinks.superseded_by contains A → A.crosslinks.supersedes must contain B; A.status must NOT be `superseded`

100% recall on the cases it covers. Zero false positives. Pattern matches `[[PROTO--ALGO-PARAM-COUPLING]]` already in the repo.

### Layer 2: Domain-scoped uniqueness

Adds a `domain:` (or `topic:`) field to ADR / CONCEPT / FEAT frontmatter — a coarse taxonomy of "what scope does this atom decide / describe."

PROTO predicate: at most one `status: stable` atom of a given type may exist for a given domain at a time. Adding a second requires either a `supersedes:` crosslink to the existing one or a justification field.

**Examples of domains:** `persistence`, `retrieval`, `identity`, `embedding-strategy`, `crosslinks`, `inbound`, `monorepo-structure`. Coarse enough that humans can pick one per atom; fine enough that genuine contradictions land in the same bucket.

Catches contradiction by **forcing it into the open**: you can't add a stable ADR on retrieval without acknowledging the existing retrieval ADR.

### Layer 3: Embedding similarity hint (PR-time, non-blocking)

CI step on PRs that touch `gks/<type>/*.md`:

1. Compute embedding of each new/changed atom (using GKS's existing `createEmbedder`)
2. Compare against embeddings of all `status: stable` atoms of the same type
3. For any pair with cosine similarity > 0.85, post a PR comment:
   > "`[[CONCEPT--FOO]]` (this PR) is highly similar to existing `[[CONCEPT--BAR]]` (sim: 0.91). If they cover overlapping scope, consider supersession."

**Non-blocking.** Reviewer judges. False positives are fine because the cost is just one extra glance at a comment.

Catches: "did you mean to supersede this atom you forgot existed?"

### Layer 4: LLM contradiction judge (PR-time, opt-in)

CI step (opt-in per repo policy or per PR label):

1. For each new/changed atom, identify the top-K most similar existing stable atoms via embedding
2. Send the diff + those atoms to a small LLM with prompt: "Does the new atom contradict any of these existing atoms? Cite specific claims."
3. Post the LLM's findings as a PR comment

Cost: ~$0.01–0.10 per PR depending on K and atom length. Opt-in because it's both expensive and probabilistic.

Catches: "two atoms claim opposite things in their body text without overlapping in vocabulary enough to trip embedding similarity."

## Scope

**In scope:**

- Define the 4 detection layers + the human rule (Layer 0)
- Specify each layer's input, output, false-positive tolerance, blocking vs. non-blocking
- Specify domain taxonomy bootstrap (Layer 2)
- Specify embedding model + similarity threshold for Layer 3
- Specify LLM provider + cost budget for Layer 4
- Order of implementation (cheapest first; later layers depend on earlier ones for accuracy)

**Out of scope (deferred):**

- Cross-type conflicts (CONCEPT vs. ADR vs. BLUEPRINT body claims) — Layer 4 covers this probabilistically; deterministic cross-type rules are a future concern
- Conflicts inside `.brain/candidates/` (per `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]`, candidates are agent-private; not validated as canon)
- Auto-rewriting atoms to resolve conflict — purely detection; resolution is human PR work
- Conflict severity scoring beyond binary (PR-comment-or-not) — could come later
- Detection of conflicts with external sources (CHANGELOGs, docs, code comments) — out of repo scope

## Why detection alone isn't enough — Layer 0 must come first

Detectors catch contradictions that are **already written**. The cheapest place to catch them is *before they're written*. A short, sharp rule in `CLAUDE.md` and the PR template:

```
PR checklist (canon atom changes):
[ ] Does this atom conflict with an existing stable atom of the same type?
    If yes — it MUST appear in `crosslinks.supersedes` AND the superseded
    atom's `status` must be flipped to `superseded` in this same PR.
[ ] Reviewer verified the above before approving.
```

Layer 0 catches the largest share of cases for the lowest cost. Detectors handle the rest.

## Why blocking vs. non-blocking matters

| Layer | Blocking? | Why |
|---|---|---|
| 0 (human rule) | n/a | Human enforcement |
| 1 (reciprocal supersession) | ✅ blocking (CI red) | Zero false positives. Always actionable. |
| 2 (domain uniqueness) | ✅ blocking (CI red) | Zero false positives once domain is set. Forces explicit supersession. |
| 3 (embedding similarity) | ❌ non-blocking | Probabilistic. Reviewer judges. |
| 4 (LLM judge) | ❌ non-blocking | Probabilistic + costly. Reviewer judges. |

Blocking layers must be **deterministic** and have **near-zero false-positive tolerance**, because false-positive blocks erode trust in the validator. Probabilistic layers post comments — comments are cheap to ignore when wrong, valuable when right.

## Source

- Existing validator rules in `src/validator/rules/`
- Existing PROTO predicates in `gks/proto/` (templates for new PROTOs)
- `[[ADR--ANTI-HALLUCINATION-RULES]]` — write-time anti-hallucination, complementary to PR-time contradiction detection
- `[[PROTO--ALGO-PARAM-COUPLING]]` — pattern for reciprocal-link PROTOs
- GKS `createEmbedder` — used by Layer 3 (already shipped in 3.6.0)
- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` — explains why candidates/ is excluded from contradiction detection

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--MSP-VALIDATOR]]
