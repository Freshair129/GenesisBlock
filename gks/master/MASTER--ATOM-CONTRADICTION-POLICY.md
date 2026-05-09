---
id: MASTER--ATOM-CONTRADICTION-POLICY
phase: 0
type: master
status: draft
tier: master
source_type: axiomatic
promoted_from: BLUEPRINT--CONTRADICTION-DETECTION-IMPL
promoted_at: 2026-05-09T08:01:00.000Z
promotion_adr: ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
vault_id: default
title: Atom contradiction policy — supersession is explicit, reciprocal, in the same PR
tags:
  - msp
  - master
  - contradiction
  - supersession
  - governance
  - instinct
crosslinks: {"references":["FRAME--KNOWLEDGE-3-TIER","ADR--MASTER-PROMOTION-CONTRADICTION-POLICY","BLUEPRINT--CONTRADICTION-DETECTION-IMPL"]}
created_at: 2026-05-09T08:01:30.000Z
---

# MASTER — Atom contradiction policy (Layer 0 supersession discipline)

## Intent

When a new atom contradicts an existing `status: stable` atom of the same type, the same PR MUST mark the conflict explicitly via reciprocal supersession; never let two stable atoms claim opposite things about the same scope.

## Why

The validator catches structural problems (schema, links, ID format) but cannot detect semantic contradiction — two atoms with valid frontmatter can still claim opposite things and both pass. Mechanical layers (PROTO predicates, embedding similarity, optional LLM judge) are scaffolding around this rule, not replacements for it. The cheapest place to catch a contradiction is at the moment the author writes it, while context is fresh and rationale is easy.

## Directives

1. Before authoring, scan `gks/<type>/` for any stable atom that overlaps the scope of your new atom.
2. If a conflict exists, in the SAME PR: (a) add the old atom's id to the new atom's `crosslinks.supersedes`, (b) add the new atom's id to the old atom's `crosslinks.superseded_by`, (c) flip the old atom's `status` to `superseded`.
3. Reciprocity is non-optional — both atoms must crosslink each other; one-sided supersession is rejected.
4. Reviewer MUST tick the supersession checklist in `.github/pull_request_template.md` before approving any PR that adds or edits an atom in `gks/<type>/`.
5. If unsure whether a conflict is real, write a short CONCEPT atom that names the divergence first, and let the supersession decision land in a follow-up.

## Apply when

A PR adds or modifies any atom in `gks/<type>/` (FRAME, CONCEPT, ADR, FEAT, BLUEPRINT, AUDIT, PROTO, MASTER). Pure additions in a new domain are exempt; edits that change a stable atom's claims are not.

## Conflicts with

(none currently — flag any future Master that proposes a different supersession discipline.)
