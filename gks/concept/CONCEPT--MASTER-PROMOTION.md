---
id: CONCEPT--MASTER-PROMOTION
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Master Block promotion — 4-of-5 dimension rule + human-in-the-loop
  proposal flow
tags: &a1
  - msp
  - master
  - promotion
  - genesis-block
  - dimensions
  - phase-e4
crosslinks: &a2
  references:
    - SPEC--GENESIS-BLOCK-MANIFEST
    - CONCEPT--TAXONOMY-V2-3
    - ADR--MASTER-PROMOTION-DOC-TO-CODE
    - ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
    - FRAMEWORK--KNOWLEDGE-3-TIER
created_at: 2026-05-13T10:00:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--MASTER-PROMOTION
  phase: 1
  type: concept
  status: stable
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Master Block promotion — 4-of-5 dimension rule + human-in-the-loop
    proposal flow
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T10:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--MASTER-PROMOTION
    phase: 1
    type: concept
    status: stable
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Master Block promotion — 4-of-5 dimension rule + human-in-the-loop
      proposal flow
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T10:00:00.000+07:00
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
---

# CONCEPT — Master Block promotion

## Why this concept exists

`[[SPEC--GENESIS-BLOCK-MANIFEST]]` §3 introduces the **5-dimension core** for a Genesis Block (Cognitive + Algo + Runbook + Concept + Params, in the v2.3 prefix vocabulary; "Frame / Algo / Proto / Concept / Param" in the EVA 4.0 vocabulary). §5 names the **4-of-5 promotion criterion**: a Genesis Block whose members fill at least 4 of these 5 roles with `status: stable` atoms becomes a *candidate* for Master Block promotion.

Until now the criterion existed only as descriptive prose. Two foundational Master atoms (`[[MASTER--MSP-DOC-TO-CODE]]`, `[[MASTER--ATOM-CONTRADICTION-POLICY]]`) were promoted manually with a hand-authored evidence ADR. As Phase E of the agentic monorepo pivot ships more Genesis Blocks, manual promotion does not scale. We need a deterministic algorithm that scans the vault, evaluates coverage per block, and proposes (but does **not** auto-write) Master atoms for human review.

## The 4-of-5 rule

Given a `GENESIS--<NAME>.md` manifest with `members.core.*` listed under five role keys, count how many of the five roles have ≥1 listed member atom that:

1. Resolves to an existing atom file on disk, and
2. Carries a `type:` consistent with the role (per the mapping below), and
3. Has `status: stable` (per `[[SPEC--GENESIS-BLOCK-MANIFEST]]` §5 promotion criterion — `status: draft` members do not contribute to the promotable count).

If that count is ≥4, the block is **promotable**. If <4, the block is *not yet* a candidate; the proposal generator reports which dimensions are missing so the next milestone can fill them.

### Role-to-prefix mapping

The SPEC §3.1 fixes the mapping. v2.3 prefix on the left, the manifest's `members.core.<key>` on the right:

| Role key (manifest) | v2.3 prefix on member atoms | EVA 4.0 source |
|---|---|---|
| `cognitive`  | `COGNITIVE--`  | `Frame::X`   |
| `algo`       | `ALGO--`       | `Algo::X`    |
| `runbook`    | `RUNBOOK--`    | `Proto::X`   |
| `concept`    | `CONCEPT--`    | `Concept::X` |
| `params`     | `PARAMS--`     | `Param::X`   |

`GENESIS--` is the manifest itself, not a sixth role. Optional roles (`guard`, `safety`, `stack`, `protocol`, `mod`, `spec`) supplement the core but do **not** count toward the 4-of-5 promotion criterion.

> Note: v2.3 retained the legacy `PARAMS--` prefix despite its non-four-letter shape (it predates the consistency push that produced `GUARD--`). The dimension analyzer treats `PARAMS--` as the canonical `params` prefix.

## When promotion fires

Promotion is **proposed**, never auto-committed:

1. An author or scheduled run invokes `msp-master-propose` against the vault.
2. The scanner walks `gks/genesis/` (and any `packages/*/gks/genesis/`), parses each `GENESIS--*.md` manifest's `members.core` block, and constructs a `DimensionCoverage` per block.
3. For each `promotable` block, the proposal generator produces a `MASTER--<id>.proposal.md` file — a draft Master atom with the canonical 5-section body schema (Intent / Why / Directives / Apply when / Conflicts with) pre-filled with a stub and `promoted_from`/`promotion_adr` placeholders.
4. With `--write`, proposals are dropped into `gks/inbound/` for a human to: write the evidence ADR, hand-edit the Master body to fit the 400-token budget (`[[PROTO--MASTER-TOKEN-CAP]]`), and *then* commit it to `gks/master/`.

We deliberately do **not** write to `gks/master/` programmatically. Master atoms are agent-instinct — every Master must pass through a human reviewer per `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]` § Decision and `[[ADR--HUMAN-REVIEW-GATES]]`. The pipeline saves the mechanical work (scanning, counting, frontmatter scaffolding) but preserves the judgement step.

## Contradiction policy summary

Per `[[MASTER--ATOM-CONTRADICTION-POLICY]]`, a proposed Master that overlaps the scope of an existing stable Master MUST be supersession-tagged in the same PR. The proposal generator emits a `# Conflicts with` section pre-populated with any existing Master ids whose tags overlap (≥1 shared tag) so the human reviewer is reminded to check for semantic overlap before committing. Mechanical contradiction detection across Masters is out of scope here; that lives in `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` (Layer 1+).

## Out of scope for this CONCEPT

- The validator rule that enforces `members.core` shape on `GENESIS--` atoms — deferred to `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]` (named but not authored in `[[SPEC--GENESIS-BLOCK-MANIFEST]]` §5).
- Automatic Master writes — see above; never auto-commit.
- Resonance Index calculation — deferred to `[[SPEC--RESONANCE-INDEX]]`.
- Storage-backend-aware promotion (Genesis Graph vs `GraphStore` vs `PgGraphBackend`) — orthogonal; the manifest declares no backend per SPEC §7.

## Connections
- [[CONCEPT--TAXONOMY-V2-3]]
- [[ADR--MASTER-PROMOTION-CONTRADICTION-POLICY]]
- [[FRAMEWORK--KNOWLEDGE-3-TIER]]

