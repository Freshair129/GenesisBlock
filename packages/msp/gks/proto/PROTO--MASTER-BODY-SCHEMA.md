---
id: PROTO--MASTER-BODY-SCHEMA
phase: 2
type: proto
status: draft
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--MASTER-BODY-SCHEMA — five required H2 sections in every Master atom body (PR-5)
tags:
  - msp
  - proto
  - master
  - schema
  - 3-tier
crosslinks: {"enforces":["FRAME--KNOWLEDGE-3-TIER"],"references":["FRAME--KNOWLEDGE-3-TIER","ADR--MASTER-PROMOTION-DOC-TO-CODE","ADR--MASTER-PROMOTION-CONTRADICTION-POLICY"]}
linked_symbols:
  - {"file":"src/validator/proto/master-body-schema.ts"}
created_at: 2026-05-09T16:05:00.000+07:00
---

# PROTO — MASTER-BODY-SCHEMA

## Rule

Every atom with `tier: master` MUST contain these five top-level H2 sections,
matched case-sensitively as exact strings (allowing trailing whitespace /
descriptive prose on the heading line):

```
## Intent
## Why
## Directives
## Apply when
## Conflicts with
```

The order is canonical (per `FRAME--KNOWLEDGE-3-TIER`), but the predicate
checks presence only, not order. The `## Conflicts with` section MAY be
empty (e.g. "(none currently)"), but its heading MUST be present so that
future atoms have a stable hook to register conflicts against.

## Schema

Reads `atomicIndex: AtomicIndexEntry[]` from the `PredicateContext`. For
each entry whose `tier === 'master'`, reads the file at
`<repoRoot>/gks/<entry.path>`, strips the leading YAML frontmatter, and
checks for the five H2 headings. A missing heading emits one
`severity: 'error'` violation per atom (one violation listing all gaps).

## Predicate

```ts
for (const entry of atomicIndex) {
  if (entry.tier !== 'master') continue
  const body = stripFrontmatter(await readFile(resolve(gksRoot, entry.path)))
  const missing = missingSections(body) // checks 5 required headings
  if (missing.length) violations.push({ atomId: entry.id, severity: 'error', ... })
}
```

Implementation: `src/validator/proto/master-body-schema.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules, alongside
other PROTO predicates).

## Severity

`error` — Master atoms are loaded directly into agent system prompts; a
malformed body breaks the loader contract that PR-6 ships
(`MASTER--<ID>` → concatenated body fragment). Better to fail at write
time than to ship a Master that breaks prompt assembly.

## Status

`draft` — promoted to `stable` once the first batch of Master atoms ships
and we've confirmed the loader (PR-6) handles the schema correctly. Until
then, violations surface in validator output but don't fail-exit per the
PROTO loader's draft policy.

## Source

`FRAME--KNOWLEDGE-3-TIER` (Master Block § "Body contract"); PR-5 of the
3-tier rollout plan.
