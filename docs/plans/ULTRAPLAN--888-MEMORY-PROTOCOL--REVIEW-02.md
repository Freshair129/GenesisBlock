# OPINION #2 — Review of ULTRAPLAN--888-MEMORY-PROTOCOL

> **Reviewer model:** claude-opus-4-7 (1M context)
> **Date:** 2026-05-17T17:30:00.000+07:00
> **Status:** Review complete — awaiting lead-dev decision on Q1/Q2/Q3
> **Subject:** `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md`

---

## TL;DR Verdict

**Approve with conditions.** The architectural reasoning is sound; the gap analysis is accurate; the doc-to-code sequencing is correct. Three structural concerns need resolution before Phase 1 work starts. One risk is underweighted. Overall this is a well-thought-out plan.

---

## 1. What the ULTRAPLAN gets right

### 1.1 Gap analysis is accurate

Section 0.2's mapping table correctly identifies that none of the three existing modules (`compressor`, `consolidator`, `EpisodicLayer`) solve cross-session distillation. The orthogonality argument in §1 (within-episode density vs. cross-time density) is the correct framing and cleanly justifies a *new* module rather than a patch to `compressor` or `consolidator`.

### 1.2 Single-Responsibility is respected

Keeping the distiller `alongside` the consolidator (Q3 recommendation) rather than replacing it is the right call. Even if consolidator is currently broken, replacing it would:
- step on Gemini's ownership boundary
- conflate two distinct responsibilities
- make rollback harder if 8-8-8 distillation proves too costly

The "alongside" model, where distiller *consumes* consolidator output, preserves clean dependency flow.

### 1.3 Configuration over magic numbers

`ADR--DISTILLATION-RATIO-CONFIGURABLE` is smart. Hardcoding 8-8-8 ratios would be an immediate tech-debt item the moment real cadence data comes in. Starting configurable (and pointing it at the new `config/` layer we're building in BLUEPRINT--CONFIG-EXTERNALIZATION) is the correct default.

### 1.4 Epistemic state vs. lifecycle status distinction

This is the conceptually sharpest part of the plan. The insight that `status` is **monotonic** (draft→active→stable→superseded, never reverting) while `epistemic_state` is **non-monotonic** (can regress on contradiction) maps to a real semantic distinction. Merging them would have been a mistake. `ADR--EPISTEMIC-VS-LIFECYCLE-STATUS` is necessary and correct.

### 1.5 Phase A-B first, then evaluate (Q2)

Correct risk management. Sphere Memory and belief revision are high-abstraction features that depend on Core Memory working well first. Real distillation telemetry from Phase B will expose assumptions about ratio, cost, and output quality that cannot be predicted from design alone.

---

## 2. Concerns

### 2.1 [HIGH] The `consolidator` breakage is a blocking risk, not a manageable one

§7 treats the `gemini/m7b-consolidator` failures (20/22 tests failing) as a risk with mitigation: "fix first OR admin-merge." The admin-merge path is a false safety valve — if consolidator is the **input** to the distiller (per Q3 recommendation), landing Phase B while consolidator is broken means Phase B is never exercised on real data in CI. The distiller would ship tested only against mocked consolidator output, which is precisely the failure mode that burned us before (mock/prod divergence in migration tests).

**Recommendation:** Make consolidator fix a **hard gate** for Phase A, not a soft option. This should be stated explicitly in §8 Acceptance.

### 2.2 [MEDIUM] LLM cost at distillation time is underspecified

§7 notes "Distiller LLM cost balloons" with mitigation "Phase B includes `costTracker` integration + budget cap." However:

- No estimate of calls per user per month is given.
- "Budget cap configurable per tenant" implies multi-tenant use, which the current MSP design does not have (it's single-identity).
- `costTracker` is mentioned but not present in any existing module or BLUEPRINT.

The cost model needs at minimum a rough calculation in Phase B's scope: *N users × M sessions/month × (sessions_per_core) distillation calls = total_tokens/month*. Without this, the budget cap is a knob with no baseline.

### 2.3 [MEDIUM] Phase A schema migration scope is underestimated

Phase A adds `domain` + `epistemic_state` to atom frontmatter and includes "migration script for existing atoms (default `domain: meta`, `epistemic_state: confirmed`)." With 386 atoms currently indexed, a migration script that touches all atom files in `gks/` risks:

1. Merge conflicts if any agents are mid-edit on atom files during migration.
2. Validator re-run on 386 files may surface latent pre-existing errors attributed to our PR.
3. The `.defaults` suffix signals "rarely touch this" — but a mass frontmatter patch touches every atom.

**Recommendation:** Either (a) defer per-atom migration to Phase A-post (schema only lands; validator emits warnings, not errors, for missing fields with default fallback), or (b) add a pre-migration check to Phase A: run `msp:validate` on `main` first, record baseline error count, compare post-migration. This baseline is essential for the "same green count" acceptance criterion.

### 2.4 [LOW] Atom ID `CONCEPT--TIERED-MEMORY-DISTILLATION` conflicts with naming decision Q1

If Q1 resolves to "native names," the atom should be `CONCEPT--TIERED-MEMORY-DISTILLATION` (good). If Q1 resolves to "literal EVA names" it would be `CONCEPT--CORE-SPHERE-MEMORY`. The ULTRAPLAN uses native names throughout §4 — which implicitly *answers* Q1 in the atom ID choice — but then lists Q1 as still open. This is a soft inconsistency: the doc has already committed to native names in its own atom plan. Make it explicit: Q1 is pre-decided as "native names + EVA aliases" in this ULTRAPLAN; §3 should say "resolved" not "open."

---

## 3. Gaps not covered

### 3.1 Storage format for Core/Sphere atoms

The plan says Core/Sphere are "atoms written via existing `retain()` API," but the GKS atom schema (`atom_schema.yaml`) defines atoms as governance/knowledge documents — not runtime memory snapshots. Writing a Core atom to `gks/` raises questions:

- Does a Core atom have a `gks/type/` path? Which cluster does it belong to? (`memory`?)
- What is its ID format? `CORE--<session-hash>--K<counter>`?
- Should Core atoms appear in `atomic_index.jsonl`? Will the validator complain about their non-standard structure?

The BLUEPRINT--TIERED-DISTILLATION (Phase 3) will need to answer this. Recommend adding a Q4 to §3: *"Storage location for Core/Sphere atoms: should they live in `gks/` (alongside governance atoms) or in `.brain/msp/` (alongside session logs)?"*

My recommendation: `.brain/msp/projects/<ns>/memory/core/` and `.brain/msp/projects/<ns>/memory/sphere/` — runtime-derived data, not governance atoms. They should not enter `atomic_index.jsonl`.

### 3.2 No rollback story for Sphere downgrades in production

The belief revision protocol (Phase D) triggers Sphere → Core downgrades. This is a **destructive read** (the Sphere's confidence state changes). There's no mention of:
- Audit log for belief revision events
- Manual override to restore a Sphere belief the system incorrectly downgraded
- Snapshot before downgrade

Suggest adding a `belief_revision_log.jsonl` alongside the Sphere file (analogous to the Genesis Graph JSONL log), written before any state mutation.

### 3.3 Relation to `BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION`

The BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION (currently in the doc-to-code queue) establishes `~/.msp/` as global identity storage and `.brain/msp/projects/<ns>/` as workspace storage. Core and Sphere memories are per-identity artifacts that transcend individual workspaces — should they live in `~/.msp/memory/` (global) rather than the workspace path? The ULTRAPLAN does not address this interaction.

---

## 4. Format / process observations

### 4.1 Source material access

§0.1 references canonical docs in `H:\My Drive\888_Memory_Protocol\`. This is a local Google Drive path that:
- Will not be accessible to any CI agent or future reviewer on a different machine.
- Is not in the monorepo.

Either copy the 3 canonical files into `docs/reference/888-memory-protocol/` under the repo, or at minimum extract the definitions needed for the CONCEPTs into the atom bodies directly (so the atoms are self-contained). The plan should not depend on a local path that isn't checked in.

### 4.2 §8 acceptance gates are incomplete

Current §8 gates are necessary but not sufficient. Suggest adding:
- `consolidator` TS errors fixed on main (hard gate — see §2.1)
- Storage path decision made (Q4 above)
- Baseline `msp:validate` green count recorded on `main` before Phase A begins

---

## 5. Open questions I'd escalate to lead-dev

1. **Q1 — Naming:** Treat as pre-decided (native names + EVA aliases) and remove from §3 as open, OR explicitly confirm. A lingering "open" blocks atom ID finalization.
2. **Q4 (new) — Core/Sphere storage path:** `.brain/msp/projects/<ns>/memory/` (workspace-local) vs. `~/.msp/memory/` (global, identity-level)? This decision should align with `BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION`.
3. **Consolidator fix timeline:** Who owns the fix — Gemini's branch or a separate hotfix? Hard gate for Phase A.
4. **LLM provider for distillation:** The distiller makes LLM calls; the ULTRAPLAN doesn't say which provider or whether it should be pluggable. This affects Phase B's API surface.

---

## 6. Execution risk summary

| Risk | Original severity | My severity | Notes |
|---|---|---|---|
| consolidator TS errors | medium (soft mitigation) | **HIGH** (hard gate) | Distiller input depends on consolidator |
| LLM cost model | medium | medium | Needs baseline calculation in Phase B |
| Mass schema migration (386 atoms) | not listed | **MEDIUM** | See §2.3 |
| Sphere storage / global vs workspace | not listed | **MEDIUM** | Interaction with BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION |
| Belief revision rollback story | not listed | medium | Suggest audit log before Phase D |
| Q1 naming bikeshed | low | low | Pre-decided in atom IDs already; formalize |
| Source material not in repo | not listed | low | Blocks future reviewers / agents |

---

## 7. Recommended pre-Phase-1 actions (in order)

1. **Fix or gate consolidator** — merge Gemini's fix or open a hotfix PR. Do not start Phase 1 until this is green.
2. **Record `msp:validate` baseline** — `npm run msp:validate` green count on `main` before any Phase A changes.
3. **Resolve Q1 in writing** — update §3 to mark Q1 as resolved. Avoids atom-naming thrash mid-stream.
4. **Add Q4 to §3** — Core/Sphere storage path decision. Coordinate with BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION author.
5. **Copy canonical docs** — `H:\My Drive\888_Memory_Protocol\` → `docs/reference/888-memory-protocol/` in the repo (or excerpt into CONCEPT atom bodies).

After those five actions, Phase 1 (CONCEPT atoms) can begin cleanly.

---

*Opinion #2 — claude-opus-4-7 (1M context)*
*2026-05-17T17:30:00.000+07:00*
