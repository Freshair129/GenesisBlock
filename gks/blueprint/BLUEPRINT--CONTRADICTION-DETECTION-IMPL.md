---
id: BLUEPRINT--CONTRADICTION-DETECTION-IMPL
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — contradiction detection implementation (5 phases, one PR per layer)
tags:
  - msp
  - validator
  - contradiction
  - blueprint
  - implementation
  - ci
crosslinks: {"references":["CONCEPT--ATOM-CONTRADICTION-DETECTION","ADR--CONTRADICTION-DETECTION-STACK","PROTO--ALGO-PARAM-COUPLING"]}
linked_symbols:
  - {"file":"CLAUDE.md"}
  - {"file":".github/pull_request_template.md"}
  - {"file":"src/validator/proto/reciprocal-supersession.ts"}
  - {"file":"gks/proto/PROTO--RECIPROCAL-SUPERSESSION.md"}
  - {"file":"src/validator/proto/domain-uniqueness.ts"}
  - {"file":"gks/proto/PROTO--DOMAIN-UNIQUENESS.md"}
  - {"file":".brain/msp/LLM_Contract/atomic_contract.yaml"}
  - {"file":".github/workflows/contradiction-similarity.yml"}
  - {"file":"scripts/msp/contradiction-similarity.mjs"}
  - {"file":".github/workflows/contradiction-judge.yml"}
  - {"file":"scripts/msp/contradiction-judge.mjs"}
  - {"file":".github/contradiction-detection.yml"}
created_at: 2026-05-08T17:34:00.000+07:00
---

# BLUEPRINT — contradiction detection implementation

```yaml
metadata:
  title: "Contradiction detection — 5-phase rollout (Layer 0..4)"
  parent_concept: CONCEPT--ATOM-CONTRADICTION-DETECTION
  parent_adr: ADR--CONTRADICTION-DETECTION-STACK

architectural_pattern: |
  One layer per phase, one PR per phase. Each phase is independently
  shippable and revertable. Cheap deterministic layers ship first;
  probabilistic layers ship last. Each phase has its own AUDIT atom.

  Phase 0 (the human rule) ships first because it has zero code cost
  and provides the most leverage per dollar.

  Phase 1 + 2 are validator-tier: they hook into the existing PROTO
  loader and run as part of `npm run msp:validate --all` and the existing
  CI test job. No new infrastructure.

  Phase 3 + 4 are CI-tier: separate GitHub Actions that post PR comments.
  They depend on the PR target being a public GitHub repo (not a fork
  without secrets, not a local pre-push hook).

phase_0_human_rule:
  goal: "Ship the policy that prevents most contradictions before any code runs."

  steps:
    0.1 CLAUDE.md
        - Add new section "## Atom contradiction policy" after the
          "Doc-to-code workflow" section
        - Content per ADR--CONTRADICTION-DETECTION-STACK § "Layer 0"
        - Reference both PROTO--RECIPROCAL-SUPERSESSION and
          PROTO--DOMAIN-UNIQUENESS as forthcoming mechanical enforcement

    0.2 .github/pull_request_template.md (NEW)
        - Standard PR sections (summary, test plan, …) matching existing
          repo convention
        - Conditional checklist for canon-atom changes:
            ## Atom contradiction checklist (only if this PR adds/edits gks/<type>/*.md)
            - [ ] No conflict with existing stable atoms of same type, OR
            - [ ] Conflicts marked via `crosslinks.supersedes` AND old atom flipped to `status: superseded` in this PR
            - [ ] Reviewer verified the above
        - Include the existing claude.ai/code session link footer

    0.3 ROADMAP.md
        - Note "Contradiction detection — Layer 0 shipped" line

    0.4 AUDIT--CONTRADICTION-DETECTION-LAYER-0
        - records what shipped + reasoning + link to ADR

  verification:
    - Read CLAUDE.md and the PR template; confirm content
    - Open a sandbox PR with an atom edit; checkout the rendered template
    - npm test still 546/546 (no code change)
    - msp:validate --all exit 0 (no atom change)

phase_1_reciprocal_supersession:
  goal: "Layer 1 PROTO blocking on broken supersession reciprocity."

  steps:
    1.1 src/validator/proto/reciprocal-supersession.ts (NEW)
        - export const id = 'PROTO--RECIPROCAL-SUPERSESSION'
        - export async function check({ atomicIndex }) → ProtoResult
        - For each atom A in atomicIndex:
            * For each B in (A.crosslinks?.supersedes ?? []):
                if !atomicIndex.has(B): violation(A, "supersedes nonexistent atom B")
                else:
                    let bAtom = atomicIndex.get(B)
                    if !(bAtom.crosslinks?.superseded_by ?? []).includes(A.id):
                        violation(A, "B does not list A in superseded_by")
                    if bAtom.status !== 'superseded':
                        violation(A, "B has status " + bAtom.status + ", expected 'superseded'")
            * Symmetric check for A.crosslinks?.superseded_by
        - return { ok: violations.length === 0, violations }

    1.2 gks/proto/PROTO--RECIPROCAL-SUPERSESSION.md (NEW)
        - frontmatter: status: stable, severity: error
        - body: rule statement + examples + cross-link to ADR

    1.3 src/validator/proto/loader.ts (MODIFIED)
        - register the new PROTO module (matches existing pattern; one-line add)

    1.4 test/validator/proto/reciprocal-supersession.test.ts (NEW)
        - ~12 tests:
            * happy path: A supersedes B, B has superseded_by A, B.status=superseded → ok
            * A supersedes B but B doesn't list it → fail
            * A supersedes B but B.status=stable → fail
            * A supersedes nonexistent B → fail
            * A.superseded_by lists B but B doesn't list A in supersedes → fail
            * A.superseded_by lists B but A.status=stable → fail
            * Multiple supersessions, all valid → ok
            * Mixed: one valid + one broken → fail with count=1
            * Empty crosslinks → ok
            * Crosslinks present but supersedes/superseded_by absent → ok
            * Self-supersession (A supersedes A) → fail
            * Cycle (A supersedes B, B supersedes A) → fail

    1.5 Backfill audit
        - Run new PROTO against existing repo via:
            npx tsx src/validator/cli.ts --all
          Expected: 0 violations because no atom currently uses supersedes
          (CONCEPT--KNOWLEDGE-LAYERS-V2 does mark CONCEPT--INBOUND-QUEUE
          as superseded — verify the latter has status: superseded after
          the inbound-deprecation BLUEPRINT phase 4 ships).

    1.6 AUDIT--CONTRADICTION-DETECTION-LAYER-1
        - records the PROTO + tests + initial backfill result

  verification:
    - npm test passes (existing + new tests)
    - PROTO appears in `npx tsx src/validator/cli.ts --all` output
    - Manually creating a broken supersession in a fixture triggers error

phase_2_domain_uniqueness:
  goal: "Layer 2 PROTO blocking on multiple stable atoms in same domain."

  steps:
    2.1 .brain/msp/LLM_Contract/atomic_contract.yaml (MODIFIED)
        - Add `domain` to optional_fields for ADR / FEAT / CONCEPT
        - Add `valid_domains` enum with initial taxonomy:
            persistence
            retrieval
            identity
            embedding-strategy
            crosslinks
            inbound
            monorepo-structure
            governance
            validator
            mcp-tooling
            session-memory
            episodic-memory
            anti-hallucination
            authority
            phase-gates
        - Add `domain_grace_period_until: 2026-12-01T00:00:00.000Z`
          (so atoms missing `domain:` are warnings during grace, errors after)

    2.2 src/validator/contract.ts (MODIFIED)
        - Load valid_domains and grace period from atomic_contract.yaml

    2.3 src/validator/rules/domain-valid.ts (NEW)
        - If atom has `domain` field: must be in valid_domains
        - Severity: error

    2.4 src/validator/rules/domain-required.ts (NEW)
        - For ADR / FEAT / CONCEPT: domain is required (severity: warning during grace, error after)

    2.5 src/validator/proto/domain-uniqueness.ts (NEW)
        - For each (type T, domain D):
            let stable = atoms where type=T and domain=D and status='stable'
            if stable.length > 1:
                violation: "multiple stable atoms in domain D of type T: " + stable.map(a=>a.id).join(', ')
        - Skip atoms without domain (covered by domain-required rule)

    2.6 gks/proto/PROTO--DOMAIN-UNIQUENESS.md (NEW)
        - frontmatter: status: stable, severity: error
        - body: rule statement + worked example + grace period note

    2.7 test files (NEW)
        - test/validator/rules/domain-valid.test.ts (~5 tests)
        - test/validator/rules/domain-required.test.ts (~5 tests, covers grace period)
        - test/validator/proto/domain-uniqueness.test.ts (~10 tests)

    2.8 Backfill — separate PR per domain cluster:
        - Read existing ADRs + CONCEPTs + FEATs; assign domain per cluster
        - One PR per domain (e.g. "domain: backfill — retrieval atoms")
          to keep diffs small and reviewable
        - Estimated ~50 atoms; ~5 backfill PRs

    2.9 AUDIT--CONTRADICTION-DETECTION-LAYER-2
        - records the schema migration + PROTO + per-domain backfill

  verification:
    - validator catches atom with `domain: invented-name` (not in enum)
    - validator warns on ADR without domain during grace period
    - PROTO catches two stable ADRs with `domain: persistence`
    - All existing repo atoms validate after backfill

phase_3_embedding_similarity:
  goal: "Layer 3 PR-comment bot using GKS embedder, threshold 0.85."

  steps:
    3.1 scripts/msp/contradiction-similarity.mjs (NEW)
        - Read changed gks/<type>/*.md files from PR diff (gh pr diff)
        - Load atomic_index.jsonl
        - For each changed atom:
            * Read body text
            * Compute embedding via @freshair129/gks createEmbedder({ provider: 'auto' })
            * For each existing atom of same type with status: stable:
                Compute similarity (cosine)
                If sim > threshold (default 0.85) and not already linked
                via supersedes/superseded_by, add to results
        - Output JSON: { pairs: [...], threshold: ..., model: ... }

    3.2 .github/workflows/contradiction-similarity.yml (NEW)
        - Trigger: pull_request paths gks/<type>/*.md
        - Steps:
            * checkout
            * setup-node 22
            * npm ci
            * node scripts/msp/contradiction-similarity.mjs > results.json
            * post-pr-comment with templated summary (use existing
              actions/github-script@v7)
            * if results.pairs is empty, skip the comment (don't spam)
        - Permissions: pull-requests: write, contents: read

    3.3 .github/contradiction-detection.yml (NEW)
        - Configurable thresholds, model selection, ignore-paths
        - Default: threshold: 0.85, model: auto, scope: gks/<type>/

    3.4 Sample-PR sanity check:
        - Open a draft PR adding a near-duplicate of an existing atom
        - Confirm bot posts a comment
        - Open a draft PR adding a wholly new atom
        - Confirm bot posts no comment

    3.5 AUDIT--CONTRADICTION-DETECTION-LAYER-3

  verification:
    - workflow runs successfully on a real PR with real similarity hit
    - PR comment renders correctly with table format
    - non-similar PR produces no comment
    - threshold tunable via .github/contradiction-detection.yml

phase_4_llm_judge:
  goal: "Layer 4 PR-comment bot via small LLM, opt-in only."

  steps:
    4.1 scripts/msp/contradiction-judge.mjs (NEW)
        - Read changed gks/<type>/*.md files
        - Reuse Layer 3 similarity output to pick top-K=5 candidates per change
        - Build prompt:
            "You are reviewing a PR adding a new atom.
             Read the new atom and the K most-similar existing stable atoms.
             For each existing atom, decide if the new atom contradicts a
             specific claim. Cite the contradicting passages.
             Output JSON per ADR--CONTRADICTION-DETECTION-STACK § Layer 4."
        - Call Anthropic API (Claude Haiku 4.5 default; configurable)
        - Cost guard: estimate tokens upfront; abort if > $0.50 budget
        - Output JSON results

    4.2 .github/workflows/contradiction-judge.yml (NEW)
        - Trigger:
            on:
              pull_request:
                types: [labeled]
              # so labeling adds the run
            jobs.contradiction-judge.if: |
              github.event.label.name == 'contradiction-check' ||
              vars.CONTRADICTION_JUDGE_DEFAULT == 'true'
        - Steps:
            * checkout
            * setup-node
            * npm ci
            * node scripts/msp/contradiction-similarity.mjs > sim.json
            * node scripts/msp/contradiction-judge.mjs sim.json > judge.json
            * post-pr-comment summarizing definite/possible findings only
        - Secret: ANTHROPIC_API_KEY
        - Permissions: pull-requests: write, contents: read

    4.3 .github/contradiction-detection.yml (MODIFIED)
        - Add: judge.model (default claude-haiku-4-5)
                judge.k_candidates (default 5)
                judge.max_cost_per_pr_usd (default 0.50)
                judge.max_cost_per_month_usd (default 50.00)

    4.4 Documentation
        - README section: "Opt-in: add 'contradiction-check' label to a PR
          to run the LLM contradiction judge"
        - Cost transparency: workflow comment includes token usage + estimated cost

    4.5 Budget tracking
        - GitHub repo Variable CONTRADICTION_JUDGE_MONTHLY_USAGE_USD
          updated by the workflow; compared against monthly cap
        - Soft cap: above 80% of monthly cap, post warning comment but still run
        - Hard cap: at 100%, skip the run with explanatory comment

    4.6 AUDIT--CONTRADICTION-DETECTION-LAYER-4

  verification:
    - PR labeled 'contradiction-check' runs the workflow
    - PR not labeled (and default off) skips the workflow
    - Cost reported in the PR comment matches a sanity calculation
    - JSON parsing handles malformed LLM output gracefully (treat as empty)
    - Monthly budget enforcement actually triggers when over cap

geography:
  - "CLAUDE.md"                                                  # MODIFIED phase 0
  - ".github/pull_request_template.md"                           # NEW phase 0
  - "ROADMAP.md"                                                 # MODIFIED phase 0
  - "src/validator/proto/reciprocal-supersession.ts"             # NEW phase 1
  - "gks/proto/PROTO--RECIPROCAL-SUPERSESSION.md"                # NEW phase 1
  - "src/validator/proto/loader.ts"                              # MODIFIED phase 1
  - "test/validator/proto/reciprocal-supersession.test.ts"       # NEW phase 1
  - ".brain/msp/LLM_Contract/atomic_contract.yaml"               # MODIFIED phase 2
  - "src/validator/contract.ts"                                  # MODIFIED phase 2
  - "src/validator/rules/domain-valid.ts"                        # NEW phase 2
  - "src/validator/rules/domain-required.ts"                     # NEW phase 2
  - "src/validator/proto/domain-uniqueness.ts"                   # NEW phase 2
  - "gks/proto/PROTO--DOMAIN-UNIQUENESS.md"                      # NEW phase 2
  - "test/validator/rules/domain-valid.test.ts"                  # NEW phase 2
  - "test/validator/rules/domain-required.test.ts"               # NEW phase 2
  - "test/validator/proto/domain-uniqueness.test.ts"             # NEW phase 2
  - "scripts/msp/contradiction-similarity.mjs"                   # NEW phase 3
  - ".github/workflows/contradiction-similarity.yml"             # NEW phase 3
  - ".github/contradiction-detection.yml"                        # NEW phase 3
  - "scripts/msp/contradiction-judge.mjs"                        # NEW phase 4
  - ".github/workflows/contradiction-judge.yml"                  # NEW phase 4
  - "gks/audit/AUDIT--CONTRADICTION-DETECTION-LAYER-0.md"        # NEW phase 0
  - "gks/audit/AUDIT--CONTRADICTION-DETECTION-LAYER-1.md"        # NEW phase 1
  - "gks/audit/AUDIT--CONTRADICTION-DETECTION-LAYER-2.md"        # NEW phase 2
  - "gks/audit/AUDIT--CONTRADICTION-DETECTION-LAYER-3.md"        # NEW phase 3
  - "gks/audit/AUDIT--CONTRADICTION-DETECTION-LAYER-4.md"        # NEW phase 4

verification_plan:
  phase_0:
    - CLAUDE.md and PR template render correctly
    - npm test: existing tests pass (no code change)
    - manual: open sandbox PR, confirm template renders

  phase_1:
    - vitest: 12 new tests pass
    - vitest: existing 546+ tests pass
    - PROTO appears in --all output
    - intentional broken supersession in fixture → CI red

  phase_2:
    - vitest: 20 new tests pass; existing tests pass
    - validator: atom with invalid domain → error
    - validator: atom missing domain → warning during grace, error after
    - PROTO: two stable ADRs same domain → CI red
    - all existing atoms validate after backfill PRs

  phase_3:
    - sandbox PR adding near-duplicate atom → bot posts comment
    - sandbox PR adding new domain → bot posts no comment
    - threshold change in config → bot uses new threshold

  phase_4:
    - PR with label 'contradiction-check' → judge workflow runs
    - PR without label and default off → judge workflow skipped
    - cost report matches expected
    - monthly cap enforcement triggers at 100% in test scenario

implementation_order:
  P0.1 CLAUDE_MD             add "Atom contradiction policy" section
  P0.2 PR_TEMPLATE           add .github/pull_request_template.md
  P0.3 ROADMAP_NOTE          one-line update
  P0.4 AUDIT_PHASE_0         AUDIT--CONTRADICTION-DETECTION-LAYER-0

  P1.1 PROTO_RECIP_TS        src/validator/proto/reciprocal-supersession.ts
  P1.2 PROTO_RECIP_MD        gks/proto/PROTO--RECIPROCAL-SUPERSESSION.md
  P1.3 LOADER_REG            register PROTO in loader.ts
  P1.4 PROTO_RECIP_TESTS     12 new tests
  P1.5 BACKFILL_VERIFY       run --all, confirm 0 violations
  P1.6 AUDIT_PHASE_1

  P2.1 CONTRACT_DOMAIN       atomic_contract.yaml: domain field + enum
  P2.2 RULE_DOMAIN_VALID     src/validator/rules/domain-valid.ts
  P2.3 RULE_DOMAIN_REQUIRED  src/validator/rules/domain-required.ts (with grace)
  P2.4 PROTO_DOMAIN_UNIQ_TS  src/validator/proto/domain-uniqueness.ts
  P2.5 PROTO_DOMAIN_UNIQ_MD  gks/proto/PROTO--DOMAIN-UNIQUENESS.md
  P2.6 DOMAIN_TESTS          ~20 new tests
  P2.7 BACKFILL_PRS          ~5 PRs assigning domain to existing atoms
  P2.8 AUDIT_PHASE_2

  P3.1 SIM_SCRIPT            scripts/msp/contradiction-similarity.mjs
  P3.2 SIM_WORKFLOW          .github/workflows/contradiction-similarity.yml
  P3.3 CONFIG_FILE           .github/contradiction-detection.yml
  P3.4 SIM_SANITY            sandbox PR validation
  P3.5 AUDIT_PHASE_3

  P4.1 JUDGE_SCRIPT          scripts/msp/contradiction-judge.mjs
  P4.2 JUDGE_WORKFLOW        .github/workflows/contradiction-judge.yml
  P4.3 CONFIG_JUDGE_FIELDS   extend .github/contradiction-detection.yml
  P4.4 BUDGET_TRACKING       monthly cap enforcement
  P4.5 README_DOC            opt-in instructions
  P4.6 JUDGE_SANITY          sandbox PR validation with label
  P4.7 AUDIT_PHASE_4
```

## Implementation notes

- **Each phase = its own PR.** Phase N must merge before Phase N+1 starts.
- **Phase 0 has zero code cost** — ship it first, this week, regardless of other plans.
- **Phase 1 has high leverage per LoC** — a few hundred lines mechanical PROTO + tests; immediately blocks the most common drift pattern (broken reciprocity).
- **Phase 2 needs schema migration** — backfill the `domain:` field across existing atoms. Do the backfill in small per-domain PRs (~10 atoms each) rather than one giant PR.
- **Phase 3 + 4 require GitHub Actions secrets** — for Phase 4, `ANTHROPIC_API_KEY` must be added to repo secrets. Without it, Phase 4 is dead weight.
- **PR template interaction** — note that the existing draft-PR convention (CLAUDE.md "Branching + PR conventions") still applies; the contradiction checklist is additive.
- **Worktree caveat (CLAUDE.md)** — subagents executing any phase must `npm ci` in the worktree.
- **Race condition lesson** — none of these phases write to `.brain/msp/projects/<ns>/inbound/` or `candidates/`; they write only to `gks/`, `src/`, `scripts/`, `test/`, `.github/`. No race risk.
- **Coordination with `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION`** — Phase 4 of that BLUEPRINT marks `CONCEPT--INBOUND-QUEUE` as `superseded`. After both BLUEPRINTs ship through their respective Phase 1 (this one) and Phase 4 (inbound deprecation), Layer 1 PROTO must pass on that supersession pair; verify in the AUDIT.

## Implementer: do NOT do

- **Don't make Layer 3 or 4 blocking.** Probabilistic layers must remain advisory comments. Blocking probabilistic layers erodes CI trust.
- **Don't combine phases into a single PR.** Even Layer 1 + Layer 2 sound related but should ship separately.
- **Don't skip the human rule (Phase 0).** It's the cheapest layer and prevents most cases. Mechanical layers without it work harder for less coverage.
- **Don't backfill `domain:` in one giant PR.** Per-domain PRs of ~10 atoms each are reviewable; a 50-atom PR is not.
- **Don't add a "domain: misc"** catch-all. Defeats the point. If an atom doesn't fit any existing domain, propose a new domain in the same PR.
- **Don't run Layer 4 on every PR.** Default-off is mandatory until you have data on cost vs. value.
- **Don't extend Layer 1 to detect "should-have-superseded" cases.** That's Layer 2/3/4 territory; Layer 1 is purely about declared-but-broken reciprocity.

## Source

- `CONCEPT--ATOM-CONTRADICTION-DETECTION` — motivation
- `ADR--CONTRADICTION-DETECTION-STACK` — decisions
- `PROTO--ALGO-PARAM-COUPLING` — template for Layer 1 PROTO
- `PROTO--PHASE-GATES` — template for Layer 2 PROTO
- `src/validator/proto/loader.ts` — registration pattern
- `@freshair129/gks` `createEmbedder` (3.6.0) — Layer 3 dependency
- Anthropic Messages API docs — Layer 4 dependency
