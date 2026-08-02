---
proposed_id: ADR--GENESISDB-DOC-GOVERNANCE
type: adr
status: proposed
tier: strategy
cluster: implementation_flow
role: "ADR — enforce doc↔code sync via a verifiability-tiered pipeline (generate / script-lint / diff-scoped agent / cron sweep) so registries can't rot again"
date: 2026-07-03
deciders: Boss
related:
  - DOC-STATUS.md
  - ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE
  - AGENT.md
---

# ADR: Documentation Governance — enforce doc↔code sync by verifiability tier

**Status:** Proposed · **Date:** 2026-07-03 · **Deciders:** Boss

## 1. Context — why this ADR exists

A 2026-07-03 drift audit of `docs/` against `main@5d4d08b` found the manual doc registry (`docs/DOC-STATUS.md`) had rotted badly in 12 days:

- **6 of 6 core narrative claims WRONG/STALE** (LogicalPlanner "dead code" → removed entirely; SDK body mismatch → fixed by untagged `HqlBody`; per-cluster ef "not wired" → shipped; HQL `IN`/`add_vector` "deferred" → shipped; anti-entropy "stub" → implemented). **Several were wrong the day the registry was written** — the doc contradicted code that already existed on 2026-06-21.
- **`docs/API_REFERENCE.md` had 3 outright-WRONG claims** (bind `0.0.0.0` vs actual `127.0.0.1`; `execute_batch` labelled NAPI-reachable when it has no `#[napi]`; `/metrics` route + collection `quant`/`ef_search`/`rerank` fields missing) — the dangerous class: a reader who follows the doc gets a broken result.
- ~46 docs (incl. all of #43–#65's merges) were **invisible** to the registry.
- **`index.d.ts` was the ONLY doc with zero drift** — because it is napi-**generated**, not hand-written.

Root causes (not "someone forgot"): (a) a **manual** registry with **no CI gate** — it was hope, not enforcement; (b) it **mixed living specs with frozen historical snapshots** (audits/reports/incidents that should never track code), so every code change nominally invalidated it; (c) it **hand-duplicated data the code already owns** (route/bin/tool lists), which drifts the instant code changes.

## 2. Decision

**Adopt a four-layer governance pipeline where each layer enforces only the claims it can actually verify, cheapest-and-deterministic first. Prose is never machine-gated; hard facts are never hand-maintained. The pipeline is enforced in CI (a failing check blocks merge) — not by convention.**

Foundational reclassification (precondition for everything): **every doc is stamped `lifecycle: living | frozen`.** `frozen` = point-in-time snapshot (all `AUDIT--P*`, `REPORT--*`, `INCIDENT--*`, `CR--*`, `METRICS-REVIEW--*`, session records) — the checker SKIPS these; they are historical record, not current truth. `living` = describes current behavior (CLAUDE.md, README, API_REFERENCE, OPERATIONS, current SPEC/ADR, the C4 map). **Only the ~10–15 living docs are gated.** This alone turns "enforce 90 docs (impossible)" into "enforce ~12 docs (tractable)".

### Layer 0 — Generate (drift made structurally impossible)
The `index.d.ts` lesson: **the doc that can't drift is the one nobody types by hand.** Anything the code already owns is generated, never authored:
- REST route table ← parsed from `src/router.rs` `.route(...)` calls.
- Bench/bin list ← parsed from `Cargo.toml` `[[bin]]` blocks.
- MCP tool list ← parsed from `mcp/server.js` `TOOLS`.
- NAPI surface ← already `index.d.ts` (napi-gen). Keep.
Generated fragments are written into the living docs between `<!-- GEN:routes START -->…<!-- GEN:routes END -->` markers; the Layer-1 script fails CI if a generated block is stale (re-run the generator).

### Layer 1 — Script lint (deterministic CI gate, blocks merge)
A `scripts/docs-validate.mjs` in the exact mold of the existing `.agents/validate-agent-registry.mjs` (wired as `npm run docs:validate`), checking only mechanically-decidable facts:
1. **No dangling refs** — every `path/like/this.rs` or `docs/X.md` mentioned in a living doc resolves to a real file. (Kills the 5 standing dangling refs.)
2. **Frontmatter schema** — every doc has `lifecycle`; every ADR `status ∈ {proposed, accepted, shipped, superseded, rejected}`; a `status: shipped` ADR must cite a merged PR number.
3. **Generated blocks are current** (Layer 0 re-run produces no diff).
4. **Code-fence sanity** — a ```hql fenced block parses via the real `HqlParser`; a ```rust doc-example compiles under a doctest harness (opt-in per fence via an attribute).
5. **Registry completeness** — every `living` doc appears in DOC-STATUS.md (or its successor index); no living doc is orphaned.
No LLM, no network, no flakiness. Catches ~40% of what the audit found, instantly and forever.

### Layer 2 — Diff-scoped semantic agent (advisory→gate, per PR)
The claims a script can't judge ("X is dead code", "Y not wired") need an LLM — but scoped to the diff, never a full re-audit (that is what a human just did manually; too expensive per PR). Mechanism = a **reverse linkage index**: each living doc declares in frontmatter
```yaml
describes: [Storage::execute_hql, src/query/hql.pest, src/router.rs]
```
On a PR, `docs-validate` computes the changed files/symbols, queries which living docs `describes` any of them, and hands **only those docs + the diff** to an agent asking one question: *"does any sentence in these docs now contradict this change? cite doc-line + code-line."* Output posts as a PR comment. **Advisory first** (LLM false-positives would get a hard gate disabled by the team); promote to blocking once its precision is trusted. A doc with no `describes` is exempt from Layer 2 (and flagged by Layer 1 to add one if `living`).

### Layer 3 — Scheduled full sweep (safety net, files issues)
A weekly cron agent = exactly the manual audit just performed, automated: read every `living` doc, verify against current code, open an issue/chip per confirmed drift. Catches semantic drift that slips Layer 2 (a claim invalidated by a change to a file the doc didn't list in `describes`). **Not a merge gate** — a backstop that also surfaces missing `describes:` links.

### Enforcement surface
- Layers 0+1: `npm run docs:validate` in a GitHub Actions job on every PR touching `docs/**` or `src/**` → **failing check blocks merge** (the only real enforcement; the rest is signal).
- Layer 2: same job, posts a comment; non-blocking until tuned.
- Layer 3: scheduled workflow (or a `schedule`-skill cloud agent).
- Local fast feedback (optional): a Claude Code `Stop`/pre-commit hook running Layer 1 so drift is caught before push, not at CI.

## 3. Options considered

| Option | Verdict | Reason |
|---|---|---|
| **Four-layer, tiered by verifiability (CHOSEN)** | adopt | Each claim-class handled by the cheapest mechanism that can actually decide it; only living docs gated; hard facts generated. Directly attacks all three root causes. |
| Keep manual DOC-STATUS.md, "try harder" | rejected | This is what just failed. Manual + no gate = hope. Non-recurring by construction won't hold across a solo/small team. |
| Pure-LLM agent audits every doc every PR | rejected | Too expensive (the manual sweep cost ~180k tokens for one pass), too flaky to hard-gate, and re-reads frozen snapshots that shouldn't be checked. Layer 2's diff-scoping is the affordable form of this. |
| Generate ALL docs from code (literate/rustdoc-only) | rejected | Works for API surface (Layer 0 already does), but strategy/positioning/ADR prose has no code source — it is the reasoning *behind* the code. Can't and shouldn't be generated. |
| Doc-in-code (doc comments as SSOT, extracted) | partial-adopt | Adopted exactly where it fits: `execute_hql` etc. get `///` comments → napi/rustdoc extraction (Layer 0). Not forced onto architectural docs. |

## 4. Consequences

**Easier:** living-doc claims become trustworthy (the WRONG-class bugs get caught at author time, not 12 days later); dangling refs and stale route/bin lists become impossible; the registry stops mixing record with truth; onboarding reads docs that provably match code.

**Harder:** every living doc must carry `lifecycle` + (for Layer 2) `describes:` frontmatter — a one-time reclassification of ~90 docs plus upkeep on new ones (Layer 1 enforces the upkeep); the generators (Layer 0) are code that must itself be maintained; Layer 2's agent adds CI cost and, until tuned, comment noise.

**Explicitly out of scope / won't do:** gating frozen snapshots; hard-gating Layer 2 on day one; enforcing prose/strategy claims by tooling (human review only); building this before the MSP/substrate living-doc set stabilizes — **sequence Layer 0+1 first** (deterministic, high ROV), defer Layer 2 until substrate docs settle so the `describes:` index isn't rebuilt mid-churn.

## 5. Rollout (if accepted)

1. **Reclassify** — stamp `lifecycle: frozen|living` across `docs/**` (bulk: prefix-based default — `AUDIT/REPORT/INCIDENT/CR/METRICS/SESSION` → frozen; rest triaged). One PR.
2. **Layer 1** — `scripts/docs-validate.mjs` + `npm run docs:validate` + CI job (dangling refs, frontmatter schema, registry completeness). Fix the audit's live findings in the same PR (API_REFERENCE 3 WRONG, CLAUDE.md 3 STALE, DOC-STATUS refresh).
3. **Layer 0** — route/bin/tool generators + `<!-- GEN -->` markers; delete the hand-typed lists.
4. **Layer 2** — add `describes:` to living docs; diff-scoped agent as an advisory PR comment.
5. **Layer 3** — weekly scheduled sweep once Layer 2 is trusted.

## 6. Action items
1. [ ] Decision on this ADR (approach) before implementation.
2. [ ] Rollout step 1 (reclassify) + step 2 (Layer 1 + fix live findings) as the first PR.
3. [ ] Steps 3–5 sequenced after the MSP/substrate living-doc set stabilizes.
