# 🟢 Proposal 06 — Update `docs/MSP_RELATIONSHIP.md` (MSP migrated off inbound queue; agent-agnostic reframe)

**Status**: merged upstream 2026-05-11.
**Applied upstream**: GksV3 commit `22f1751`.

## Why

`docs/MSP_RELATIONSHIP.md` in `Freshair129/GksV3` is now ~6 weeks behind the actual `Freshair129/msp` repo. The doc still describes the old inbound-queue + `/submit-memory` workflow that MSP retired in Phase 3 (commit `7eff62b feat: delete msp_propose + inbound infrastructure (Phase 3)`), and conflates two different MSPs (the EVA-side Python MSP-v9.1 vs this TypeScript passport-orchestrator). Since MSP just declared itself **agent-agnostic** in [Freshair129/msp#65](https://github.com/Freshair129/msp/pull/65) (now merged), the doc reads as if MSP is EVA-coupled — the opposite of the new direction.

Fixing this matters because:

1. New downstream readers (Memory OS implementers building on GKS) hit `MSP_RELATIONSHIP.md` early and form an outdated mental model.
2. The doc is the canonical "why GKS leaves room for MSP" reference; if MSP's shape changes, the contract description should follow.
3. The two-MSP confusion is a real onboarding friction — `MSP-v9.1` (Python, EVA-internal) and `Freshair129/msp` (TypeScript, public passport) are not the same thing.

## What

Documentation-only PR. Four targeted edits to `docs/MSP_RELATIONSHIP.md`:

### 1. Add a "Which MSP?" table near the top

Right after the existing TL;DR. Disambiguates the two implementations:

```markdown
## Which MSP?

This doc was originally written when "MSP" meant EVA's MSP-v9.1. There are
now two implementations in the wild that both honor the GKS contract:

| Name | Repo | Language | Shape |
|---|---|---|---|
| **MSP-v9.1** | EVA-internal | Python | Biological-consolidation Memory OS — RI levels, RMS affect, Session→Core→Sphere cascade |
| **Freshair129/msp** | [public](https://github.com/Freshair129/msp) | TypeScript | Passport-orchestrator — agent-agnostic; plugs into Claude Code / Gemini CLI / Antigravity / EVA / openclaw / Hermes |

Both implement the gatekeeper-above-GKS contract. The rest of this doc
applies to **either**, except where noted.
```

### 2. Replace the `/submit-memory` + inbound-queue workflow

The "What MSP brings on top → Workflow" section currently reads:

```
Agent → /submit-memory → inbound queue → human review → promote → gks/
```

`Freshair129/msp` removed `/submit-memory`, the inbound queue, `msp_propose`, and `scripts/msp/propose.mjs` in Phase 3 of `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION` (merged 2026-05-04). The replacement workflow:

```
Agent → msp_candidate (MCP tool) → .brain/msp/projects/<ns>/candidates/ → human PR → gks/
```

EVA's MSP-v9.1 may still use the inbound shape; this should be flagged as the legacy path and the candidates pipeline as the new default for `Freshair129/msp`.

### 3. Update the CLI bullets

Current text lists:

```
- npm run msp:propose / review / promote / validate / index
```

`Freshair129/msp` package.json no longer ships `msp:propose`, `msp:review`, or `msp:promote`. Current scripts (per `package.json` on `main`):

```
msp:index, msp:check-links, msp:verify, msp:validate, msp:backlinks,
msp:run-task, msp:master, msp:graph, msp:hotfix:{open,list,close,check}
```

Mention that EVA's MSP-v9.1 may still expose `msp:propose` etc.; this table belongs to the public TS implementation.

### 4. Reframe `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` as EVA-specific

The table in "What MSP brings on top → Process artifacts" lists:

```
MSP-IMP- (P3 plan) → MSP-TSK- (P4 task) → MSP-ACT- (P5 action) → MSP-WKT- (P6 walkthrough)
```

Per GKS `SCOPE.md` line 135 ("Workflow / governance" section): the `FRAMEWORK_MASTER_SPEC.md` from the EVA project documents this layer; GKS does not implement it. These IDs are EVA process artifacts, not generic MSP ones. `Freshair129/msp` deleted `CORE_FRAMEWORK_MASTER_SPEC.md` from its repo specifically to break this coupling (see `gks/audit/AUDIT--ARCH-DOC-CLEANUP.md`).

Suggest:
- Move the section under a "EVA-specific process artifacts" subheading
- Add a note: "These are EVA's process IDs (per its `FRAMEWORK_MASTER_SPEC.md`). The public `Freshair129/msp` implementation does not define an equivalent process-ID convention; that's left to the consuming project."

## Compat

Doc-only. No code change. No breakage in either MSP implementation. The new "Which MSP?" section is purely additive.

## Test

N/A. Optionally: add a CI check that `Freshair129/msp` references in this doc resolve (link-check), but that's tangential.

## Atom reference

- [`AUDIT--ARCH-DOC-CLEANUP`](https://github.com/Freshair129/msp/blob/main/gks/audit/AUDIT--ARCH-DOC-CLEANUP.md) — drove the agent-agnostic reframe
- [`CONCEPT--AGENT-AGNOSTIC`](https://github.com/Freshair129/msp/blob/main/gks/concept/CONCEPT--AGENT-AGNOSTIC.md) — declares MSP is pluggable into any cognitive-layer client
- [`BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION`](https://github.com/Freshair129/msp/blob/main/gks/blueprint/BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION.md) — the migration that removed `/submit-memory`
- [`ADR--AGENT-WRITE-BOUNDARIES`](https://github.com/Freshair129/msp/blob/main/gks/adr/ADR--AGENT-WRITE-BOUNDARIES.md) — replacement contract: agents write only via `msp_candidate` → `.brain/.../candidates/`

## Issue body for relay

The full body to paste into a `Freshair129/GksV3` issue is in `upstream/gks-proposals/SUBMISSION.md` § "Issue 6 of 6" once that table is updated. Title:

```
docs: update MSP_RELATIONSHIP.md — MSP migrated off inbound queue (Phase 3) and is now agent-agnostic
```
