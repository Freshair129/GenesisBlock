# 🟢 Proposal 07 — Clarify `FRAME--` scope (canonical narrow vs Memory-OS actual broad)

**Status**: merged upstream 2026-05-11.
**Applied upstream**: GksV3 commit `22f1751`.

## Why

`docs/KNOWLEDGE-TYPES.md` defines `FRAME--` narrowly:

> `FRAME--` · code standards / framework rules
> Use for: "all DB calls go through repositories", "components ≤ 500 LOC", lint policy.
> Don't use for: runtime behavioural constraints — those are `GUARDRAIL--`.

But every Memory OS layer that adopts GKS in practice ends up using `FRAME--` for a **broader** "reusable structural pattern" sense — including architectural, governance, taxonomic, and methodological frameworks. Concrete evidence from `Freshair129/msp` (a public Memory OS using `@freshair129/gks` 3.6.0):

| MSP atom | Sense | Fits canonical FRAME-- definition? |
|---|---|---|
| `FRAME--MSP-ARCHITECTURE-V2` | architectural | ❌ not code standards |
| `FRAME--AUTHORITY-MATRIX` | governance | ❌ not code standards |
| `FRAME--KNOWLEDGE-3-TIER` | taxonomic | ❌ not code standards |
| `FRAME--PHASE-GOVERNANCE` | process methodology | ❌ not code standards |
| `FRAME--SCALING-LEVELS` | decision framework | ❌ not code standards |
| `FRAME--CROSSLINKS-VOCABULARY` | taxonomy reference | ❌ not code standards |
| `FRAME--SYMBOL-GRAPH` | architectural | ❌ not code standards |

**Every** `FRAME--` atom in `gks/frame/` on `Freshair129/msp` main is broader than the canonical narrow definition. The narrow definition is technically violated by an entire functioning Memory OS — that's a signal the canonical needs to widen, not that consumers should reshape their atoms.

Furthermore, downstream users are starting to want **business methodology frameworks** in atoms — 5Es Framework (instructional design), JTBD (Jobs To Be Done, product methodology), Design Thinking, etc. These are:

- Reusable thinking patterns (multi-project)
- Not code standards (no `FRAME--` per canonical narrow)
- Not single decisions (no `ADR--`)
- Not external regulations (no `CONSTRAINT--`)
- Not interaction contracts (no `PROTOCOL--`)

There is currently **no canonical home** for them.

## What

Pure documentation change to `docs/KNOWLEDGE-TYPES.md`. Three options for the GKS maintainer, in order of how much the canonical type system shifts:

### Option A — broaden `FRAME--` definition (smallest change)

Replace lines 121-124 of `docs/KNOWLEDGE-TYPES.md`:

```diff
 ### `FRAME--` · code standards / framework rules
-- **Use for:** "all DB calls go through repositories", "components ≤ 500 LOC", lint policy.
-- **Don't use for:** runtime behavioural constraints — those are `GUARDRAIL--`.
+- **Use for:** reusable structural patterns — code standards
+  ("all DB calls go through repositories", lint policy),
+  architectural frames ("3-tier knowledge model", "Tri-Brain architecture"),
+  governance frames ("authority matrix", "phase governance"),
+  taxonomic frames ("crosslinks vocabulary"),
+  and methodology frameworks ("5Es", "JTBD", "Design Thinking").
+- **Distinguishing question:** is this a *reusable pattern* (vs a one-off decision)?
+  If yes → `FRAME--`. If a single decision → `ADR--`. If external regulation → `CONSTRAINT--`.
+- **Don't use for:** runtime behavioural constraints — those are `GUARDRAIL--`.
+  Don't use for: single decisions — those are `ADR--`.
 - **Phase:** P2.
```

**Compat**: Existing narrow-sense FRAME-- atoms stay valid; broader-sense atoms become officially blessed.

### Option B — add new type `METHODOLOGY--` (cleaner semantics, more types)

Keep FRAME-- narrow. Add a new Cluster-1 type:

```diff
 ### `FRAME--` · code standards / framework rules
 ... (unchanged)

+### `METHODOLOGY--` · reusable thinking pattern / business framework
+- **Use for:** instructional design (5Es), product (JTBD, Design Thinking),
+  process (PDCA, OODA), governance frames, architectural frames.
+- **Don't use for:** code-level conventions (use `FRAME--`),
+  external regulation (use `CONSTRAINT--`),
+  single decisions (use `ADR--`).
+- **Distinguishing question:** is this a *thinking / structural pattern* that can be applied
+  across projects, distinct from any single codebase's conventions?
+- **Phase:** P2.
+- **Examples:** `METHODOLOGY--5ES`, `METHODOLOGY--JTBD`, `METHODOLOGY--PHASE-GOVERNANCE`.
```

**Compat**: Existing FRAME-- atoms with architectural/governance senses can be migrated to METHODOLOGY-- over time, or left as-is (broader FRAME-- senses become deprecated but allowed). Higher cost, cleaner semantics.

### Option C — clarify in canonical that FRAME-- has two valid interpretations

Same body change as Option A, but explicitly call out the canonical-vs-broad split with a disambiguation note:

```markdown
### `FRAME--` · code standards / structural framework rules

**Canonical (narrow):** code standards — "all DB calls go through repositories",
"components ≤ 500 LOC", lint policy.

**Memory-OS-adopted (broad):** any reusable structural pattern —
architectural frames, governance frames, taxonomies, methodologies.

**Both interpretations are valid in GKS.** Pick the narrow form for engineering-only repos;
broader form for projects with a Memory OS layer above.
```

**Compat**: Most conservative — explicitly blesses both readings.

## Recommendation

**Option A** is the cheapest path to consistency. Memory OS adopters already use FRAME-- broadly; the doc should follow practice rather than push for a refactor across consumers. Option B is cleaner but more disruptive.

## Compat

All three options are doc-only. No code change in GKS. No breakage in any consumer (atoms with `type: frame` continue to validate as before).

## Test

N/A (documentation only). Optionally: re-run upstream GKS examples to confirm no atom-template under `examples/atom-templates/` assumes the narrow definition.

## Atom reference

- The MSP atoms cited above all live in [`Freshair129/msp/gks/frame/`](https://github.com/Freshair129/msp/tree/main/gks/frame)
- Related sibling proposal: [`06-msp-relationship-update.md`](./06-msp-relationship-update.md) (also documents MSP↔GKS drift after MSP went agent-agnostic)
- Drove from architecture-doc cleanup audit: [`AUDIT--ARCH-DOC-CLEANUP`](https://github.com/Freshair129/msp/blob/main/gks/audit/AUDIT--ARCH-DOC-CLEANUP.md)

## Issue body for relay

The full body to paste into a `Freshair129/GksV3` issue is in `upstream/gks-proposals/SUBMISSION.md` § "Issue 7 of 7" once that section is added. Title:

```
docs: broaden FRAME-- definition (or add METHODOLOGY--) — canonical narrow vs Memory-OS practice
```
