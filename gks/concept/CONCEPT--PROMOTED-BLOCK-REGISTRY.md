---
id: CONCEPT--PROMOTED-BLOCK-REGISTRY
phase: 1
type: concept
status: draft
tier: genesis
source_type: axiomatic
vault_id: default
title: Promoted-Block Registry — first-class runnable status for Master-tier Genesis Blocks
tags:
  - msp
  - master
  - promotion
  - registry
  - genesis-block
  - runtime
  - phase-f1
crosslinks: {"references":["CONCEPT--MASTER-PROMOTION","SPEC--GENESIS-BLOCK-MANIFEST","CONCEPT--GENESIS-BLOCK-RUNTIME","FRAMEWORK--KNOWLEDGE-3-TIER","ADR--MASTER-PROMOTION-DOC-TO-CODE","ADR--HUMAN-REVIEW-GATES"]}
created_at: 2026-05-14T05:00:00.000+07:00
---

# CONCEPT — Promoted-Block Registry

## Why this concept exists

Phase E4 (`BLUEPRINT--MASTER-PROMOTION-PIPELINE`) shipped the *proposal* side of Master Block promotion: `msp-master-propose` writes a `MASTER--<id>.proposal.md` into `gks/inbound/` when a Genesis Block fills 4-of-5 dimensions. Phase E5 (`BLUEPRINT--GENESIS-BLOCK-RUNTIME`) shipped the *executor* side: `msp-genesis-exec <blockId>` loads the manifest, composes the prompt, and dispatches it.

What's missing is the **glue between promotion and runtime**. A `MASTER--<id>.md` file that a human has accepted and committed is, by itself, just a markdown file. The executor has no way to know "this block has graduated to Master tier" — there's no signal that the block deserves a different runtime treatment (e.g. a more capable model, a different logging tier, audit emission).

This CONCEPT names the missing artefact: a **Promoted-Block Registry** — a small, append-only JSONL log at `gks/master/registry.jsonl` (gitignored, derived state) that tracks which Genesis Blocks have been promoted to Master status. The registry is the *index* over the `gks/master/MASTER--*.md` atoms; the atoms remain the source of truth, but the registry gives the runtime an O(1) lookup.

## What the registry is

One JSONL row per promotion event:

```jsonl
{"block_id":"IDENTITY-ENGINE","promoted_at":"2026-05-14T04:00:00.000Z","promotion_pr":"#125","status":"active"}
```

Fields:

- `block_id` — the bare slug (matches `GenesisManifest.id`, the same value passed to `executeBlock()`).
- `promoted_at` — ISO UTC timestamp of the promotion (the moment `msp-master-propose apply` accepted the proposal).
- `promotion_pr` — optional human-readable reference to the PR or commit that authored the Master atom + evidence ADR. Free-form string, not parsed.
- `status` — `'active' | 'archived'`. `archived` is reserved for future "this Master was superseded" events; F1 only writes `active` entries.

The registry is **gitignored** (added to `.gitignore` in this PR). Two reasons:

1. **Derived state.** The Master atom at `gks/master/MASTER--<id>.md` is the source of truth — its frontmatter carries `promoted_from`, `promoted_at`, and `promotion_adr`. The registry is a fast index, not authoritative. Anyone can rebuild it by walking `gks/master/`.
2. **Avoids merge conflicts.** Multiple PRs proposing/applying different Masters would all touch the same JSONL file; gitignoring it sidesteps that. The promotion event is captured in git via the `MASTER--*.md` atom commit; the registry just makes that event runtime-discoverable on each machine.

## Why decouple promotion from runtime

`CONCEPT--MASTER-PROMOTION` already enforces the human-in-the-loop gate (`ADR--MASTER-PROMOTION-DOC-TO-CODE`): `msp-master-propose` never writes to `gks/master/`. F1 preserves that — the proposal still lands in `gks/inbound/`, and a human still has to write the evidence ADR and the body of the Master atom.

What F1 changes is the *follow-through*. The previously implicit "and then I move the proposal to `gks/master/`" step becomes an explicit, scripted action: `msp-master-propose apply <proposalPath>`. That subcommand:

1. Reads the proposal,
2. Stamps `tier: master`, `promoted_at`, `promoted_from` into the frontmatter,
3. Writes the file to `gks/master/MASTER--<id>.md`,
4. Appends a row to `gks/master/registry.jsonl`,
5. Consumes the proposal (deletes or renames it to `.applied`).

The apply step is still human-triggered — there's no auto-promotion. But once it runs, the runtime sees the registry and behaves differently:

- `executeBlock()` checks `findActiveMaster(blockId)` before calling `dispatch()`.
- On hit, the result carries `from_master: true`.
- On hit, the default tier biases upward: a Master block is presumed important enough to warrant **T2 minimum** (callers can still pass `tier: 'T3'` to force, or override the inference).

## What this CONCEPT is not

- **Not a contradiction detector.** F1 does not check whether two Master atoms conflict. That lives in `BLUEPRINT--CONTRADICTION-DETECTION-IMPL` Layer 1+.
- **Not a publishing pipeline.** The registry is a local file. There is no remote sync, no "registry server", no cross-machine consensus.
- **Not a replacement for the atom.** Deleting the registry never destroys data — the runtime degrades to "this block is treated as a normal Genesis Block", but `gks/master/MASTER--<id>.md` remains the canonical record.
- **Not a status field.** The `status: 'active' | 'archived'` value lives only in the registry; the Master atom's frontmatter `status:` is the standard atom lifecycle (`draft / stable / deprecated / superseded`) and is unaffected.

## Acceptance signals

After F1 ships, the following should be true:

1. A human can run `msp-master-propose apply gks/inbound/MASTER--FOO.proposal.md` and end up with `gks/master/MASTER--FOO.md` + an entry in `gks/master/registry.jsonl`.
2. `msp-genesis-exec FOO --prompt="…"` invoked against a Genesis Block whose Master has been applied produces an `ExecuteResult` with `from_master: true`.
3. The same command against a block with no Master in the registry produces `from_master` undefined (or absent), and the runtime behaves exactly as it did before F1.
4. The registry file is gitignored and never appears in `git status`.
