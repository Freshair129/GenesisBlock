# Review #01 — ULTRAPLAN--888-MEMORY-PROTOCOL

> **Reviewer:** Claude Code (model: **Opus 4.7 (1M context)** — `claude-opus-4-7[1m]`)
> **Role:** Lead-dev review pass (Boss seat)
> **Reviewed file:** `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md`
> **Date:** 2026-05-17T08:30:00+07:00
> **Verdict:** **Conditionally approved** — proceed after two fixes below.

---

## 0. Verification of plan's load-bearing claims

Before reviewing recommendations, I checked the plan's factual claims against the working tree.

| Claim in ULTRAPLAN | Verified? | Evidence |
|---|---|---|
| Source material at `H:\My Drive\888_Memory_Protocol\` exists | ✅ | `ls` returns `README.md`, `configs/`, `docs/`, `from_THA01/`, `specs/`, `state/` |
| `packages/msp/src/orchestrator/compressor/` exists with `tokens.ts`, `trim.ts`, `resummarise.ts` | ✅ | All files present |
| `packages/msp/src/orchestrator/consolidator/` exists (Gemini WIP) | ✅ | All files present (`score.ts`, `summarise.ts`, `boundary.ts`, `llm.ts`, etc.) |
| All cross-linked CONCEPT atoms exist: `TAXONOMY-V2-3`, `CONTEXT-COMPRESSION`, `EPISODE-RETENTION`, `RESOLUTION-GRADIENT` | ✅ | All four files present in `gks/concept/` |
| `epistemic_state` not yet used anywhere in `gks/` | ✅ | `grep -r 'epistemic_state\|epistemic-state' gks/` → no matches. Schema extension is genuinely net-new ground. |
| `gemini/m7b-consolidator` TS errors "still on main" (Risk #1) | ⚠️ **Stale.** | `git log packages/msp/src/orchestrator/consolidator/` shows `160ac66 fix(msp/consolidator): resolve M7b type inconsistencies and centralize config` on top of the WIP commit. The "broken on main" assumption may already be false. See Issue #1 below. |

**Net:** Plan's factual foundations are sound. The orthogonality argument in §1 (within-episode vs cross-time) genuinely holds — compressor and consolidator do not fill the cross-session distillation gap. 8-8-8 deserves its own module, its own ADR(s), and its own atoms, as proposed.

---

## 1. Recommendations on the three open questions

All three of the plan's recommended answers are sound. I would accept each as written.

### Q1 — Naming → **native names + EVA aliases** ✅
Right call. Cognitive_system has many readers who never touched EVA; consistency with existing GKS vocabulary wins. Carrying `aliases: [Consciousness, Session, Core, Sphere]` on the CONCEPT atom preserves searchability without forcing dual-vocabulary on the codebase.

Suggested concrete mapping (commit it to `CONCEPT--TIERED-MEMORY-DISTILLATION` to lock the terms):

| EVA | Cognitive_system |
|---|---|
| Consciousness | Trace (transient) |
| Session Memory | Episode |
| Core Memory | Narrative |
| Sphere Memory | Identity |

### Q2 — Scope → **Phase A-B first, evaluate, then C-D** ✅
Right call. The Session→Core distiller is the new primitive; Sphere and belief-revision design will be measurably better-informed once we have real telemetry on:
- How often a Core actually forms (8 episodes is the default — real cadence may be 3 or 20)
- LLM cost per Core distillation
- Whether the 4-Pillar pipeline yields qualitatively useful Cores at all

Without that data, Phase C-D design is speculative.

### Q3 — Relationship to consolidator → **sit alongside** ✅
Right call, with one caveat: the boundary must be **atomized**, not just stated in a planning doc. See Issue #2 below.

The data-flow contract should be:

```
turns ──► consolidator (within-session) ──► episodes ──► distiller (cross-session) ──► Core/Sphere
```

Each arrow is a stable interface; each box has a single owner.

---

## 2. Issues to resolve before Phase 1 begins

### Issue #1 — Stale risk claim (must fix)

**Where:** §7 Risks, row 1 + §8 Acceptance gate, checkbox 4.

**Problem:** Both reference "gemini/m7b-consolidator TS errors still on main." But commit `160ac66 fix(msp/consolidator): resolve M7b type inconsistencies and centralize config` is already on top of the WIP commit.

**Action:**
1. Run `npm run typecheck` on a clean `main` checkout.
2. If green → delete Risk row 1, delete Acceptance gate item 4, note the resolution in §0.
3. If still failing → keep the risk, but rephrase to name the specific failing files so the fix is scoped, not vague.

### Issue #2 — Missing ADR for the Q3 decision (must fix)

**Where:** §4 Phase 2 lists three ADRs:
- `ADR--ADOPT-888-AS-INSPIRATION`
- `ADR--DISTILLATION-RATIO-CONFIGURABLE`
- `ADR--EPISTEMIC-VS-LIFECYCLE-STATUS`

**Problem:** None of these captures the **alongside** decision. That decision shapes module boundaries, ownership, and interface contracts — it is *exactly* the kind of decision an ADR is supposed to record. Leaving it as a planning-doc footnote means future contributors will rediscover and relitigate it.

**Action:** Add a fourth ADR to Phase 2:

> **`ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY`**
> Distiller is a separate module from consolidator. Distiller *consumes* consolidator output (consolidated episodes); it does not extend or replace it. Rationale: orthogonal axes (within-session vs cross-session); avoids stepping on Gemini's ownership of consolidator; keeps each module's scope narrow.

---

## 3. Smaller suggestions (not blockers)

### Suggestion A — Split Phase 1 into two PRs

Phase 1 currently bundles **4 CONCEPT atoms ≈ 270 lines** into one PR. That is enough material that review will either be shallow or slow. Suggest splitting along a clean seam:

| PR | CONCEPTs | ADRs | Theme |
|---|---|---|---|
| 1a (Architecture) | `TIERED-MEMORY-DISTILLATION`, `MEMORY-DOMAINS` | `ADOPT-888-AS-INSPIRATION`, `DISTILLATION-RATIO-CONFIGURABLE`, `DISTILLER-VS-CONSOLIDATOR-BOUNDARY` | "How the pipeline works" |
| 1b (Confidence model) | `EPISTEMIC-STATES`, `BELIEF-REVISION` | `EPISTEMIC-VS-LIFECYCLE-STATUS` | "How beliefs change over time" |

1b can land after 1a is stable, which also means Phase A (schema extension) can start on top of 1a without waiting for the full Phase 1.

### Suggestion B — Pin the default ratio in a single place

The plan says `8` is configurable. Good. But the codebase will likely accumulate `8` as a magic literal in multiple places (validator ranges, default config, doc examples, test fixtures). Worth specifying in `BLUEPRINT--TIERED-DISTILLATION` that:

- `msp.config.distillation.sessions_per_core` (default `8`)
- `msp.config.distillation.cores_per_sphere` (default `8`)

are the **only** authoritative sources. Validator and tests read from there. Doc examples reference the constants symbolically (`${SESSIONS_PER_CORE}`) — never bake the integer literal.

### Suggestion C — Phase E (MCP integration) should specify telemetry

`BLUEPRINT--TIERED-DISTILLATION` should commit to emitting at least:
- `distiller.session_to_core.count` (per tenant, per domain)
- `distiller.core_to_sphere.count`
- `distiller.belief_revision.downgrade.count`
- `distiller.llm.cost_usd` (rolled into existing `costTracker`)

Without these, the "evaluate then proceed to C-D" gate from Q2 is unanswerable in practice.

### Suggestion D — Out-of-scope section is good; one addition

Add: **"This ULTRAPLAN does not modify the existing `compressor` 3-tier pipeline."** Worth stating explicitly so a future contributor doesn't try to unify them.

---

## 4. What I'm explicitly *not* worried about

- **Atom volume.** Phase 1's 4 CONCEPTs + Phase 2's 3-4 ADRs + Phase 3's 2 BLUEPRINTs is a reasonable doc-to-code prelude for a ~1350 LoC code roadmap. Ratio is healthy.
- **8 as the magic number.** Already addressed via `ADR--DISTILLATION-RATIO-CONFIGURABLE`. Start at 8, tune from data.
- **Sphere/atom authority conflict.** Already addressed via `ADR--EPISTEMIC-VS-LIFECYCLE-STATUS` (atom = canonical doc lifecycle; Sphere = derived wisdom, subordinate; on contradiction, atom wins, Sphere flagged `contested`). This is the right hierarchy.
- **Naming bikeshed.** The "native + aliases" recommendation kills this risk.

---

## 5. Sign-off conditions (updated)

I am ready to approve Phase 1 start when:

- [ ] **Issue #1** resolved — `npm run typecheck` run on clean `main`, risk/gate language updated based on the actual result.
- [ ] **Issue #2** resolved — `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` added to Phase 2 list.
- [ ] Boss confirms Q1/Q2/Q3 (recommended answers all stand).
- [ ] Phase 1 split per Suggestion A (or explicit decision not to split, with reason).

Suggestions B-D are not blockers — adopt or skip per your judgment.

---

## 6. Bottom line

The plan correctly identifies a real architectural gap (cross-session distillation + confidence-as-data) and proposes the right shape of solution. The doc-to-code phasing follows CLAUDE.md mandatorily; the risks list is honest; the §1 root-cause analysis genuinely engages with "why not extend the existing module" rather than hand-waving.

After the two issue fixes above, this is ready to execute.

---

*— Review #01 by Claude Code, model `claude-opus-4-7` (Opus 4.7, 1M context window).*
