---
id: AUDIT--GENESIS-BLOCK-MEMBERSHIP-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — PROTO--GENESIS-BLOCK-MEMBERSHIP — what shipped
tags: &a1
  - msp
  - proto
  - genesis
  - knowledge-block
  - audit
crosslinks: &a2
  references:
    - PROTO--GENESIS-BLOCK-MEMBERSHIP
    - SPEC--GENESIS-BLOCK-MANIFEST
    - CONCEPT--PROTO-PATTERN
    - CONCEPT--TAXONOMY-V2-3
created_at: 2026-05-14T18:35:00.000+07:00
aliases: &a3
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--GENESIS-BLOCK-MEMBERSHIP-PROTO
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: AUDIT — PROTO--GENESIS-BLOCK-MEMBERSHIP — what shipped
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T18:35:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--GENESIS-BLOCK-MEMBERSHIP-PROTO
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: AUDIT — PROTO--GENESIS-BLOCK-MEMBERSHIP — what shipped
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T18:35:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--GENESIS-BLOCK-MEMBERSHIP-PROTO
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: AUDIT — PROTO--GENESIS-BLOCK-MEMBERSHIP — what shipped
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T18:35:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: aws_secret
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# AUDIT — [[PROTO--GENESIS-BLOCK-MEMBERSHIP]]

## Scope

Closes the follow-up `[[SPEC--GENESIS-BLOCK-MANIFEST]]` §5 explicitly deferred:
a PROTO that machine-enforces the Block Manifest membership + status
contract. Until now §2.2 / §3 / §4.2 of the SPEC were descriptive only —
authors and reviewers obeyed them by convention.

## What shipped

| File | What |
|---|---|
| `gks/proto/[[PROTO--GENESIS-BLOCK-MEMBERSHIP]].md` | The rule atom. `status: draft`, `severity: error`, `crosslinks.enforces: [[[SPEC--GENESIS-BLOCK-MANIFEST]]]`. |
| `packages/msp/src/validator/proto/genesis-block-membership.ts` | The predicate + exported helpers (`checkManifest`, `collectMembers`, `parseFrontmatter`, `statusRank`, `CORE_ROLES`, `ROLE_TYPE`, `STATUS_ORDER`, `ID_PATTERN`). |
| `packages/msp/test/validator/proto/genesis-block-membership.test.ts` | 22 tests — helpers, `checkManifest` across all four checks, and the disk-reading predicate. |

## What the predicate enforces

For every atom with `type: genesis`, reads the manifest frontmatter and checks:

1. **Block fields** — `members`, `manifest_version`, `daci.driver` present.
2. **Five-dimension core** — `members.core` lists all of cognitive / algo /
   runbook / concept / params, each with ≥1 id.
3. **Aggregation grammar** — every `members.*` id is canonical, resolves to
   an existing atom, and that atom's `type:` matches its role.
4. **Status cascade** — block `status` equals `min(member statuses)` under
   `stub < raw < draft < active < stable`; a deprecated/superseded member
   forces a terminal status.

## Known follow-ups

- **Promotion to `stable`** is gated on the first real `GENESIS--<NAME>`
  Block Manifest being authored (e.g. `[[GENESIS--IDENTITY-ENGINE]]`, itself
  blocked on several unauthored member atoms). Until then the predicate
  passes trivially on `--all` — there are zero `type: genesis` atoms.
- **The 4-of-5 promotion criterion** (SPEC §5: ≥4 core dimensions filled
  with `status: stable` atoms → Master Block promotion candidate) is *not*
  enforced here. It is an advisory signal, not a hard rule, and belongs in
  the Master-promotion pipeline rather than the manifest validator.
- **Pre-existing PROTO path drift** — 14 of the 16 PROTO atoms in
  `gks/proto/` carry stale pre-monorepo `linked_symbols` paths
  (`src/validator/proto/…` instead of `packages/msp/src/validator/proto/…`)
  and are silently dropped by the loader. This PROTO uses the correct
  monorepo path and loads. The drift fix is tracked separately — out of
  scope here.

## Verification

- `npx vitest run test/validator/proto/genesis-block-membership.test.ts` —
  22/22 pass.
- `npm run typecheck` — clean.
- `npm run msp:validate` — the PROTO + AUDIT atoms validate; the predicate
  loads (correct `linked_symbols` path) and passes (no `type: genesis`
  atoms to check).

## Connections

- [[CONCEPT--PROTO-PATTERN]]
- [[CONCEPT--TAXONOMY-V2-3]]
