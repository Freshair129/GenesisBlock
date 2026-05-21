# Workflow — the doc-to-code loop end-to-end

How a feature travels from idea to merged code under GKS, mapped onto
the master-spec phases (`FRAMEWORK_MASTER_SPEC.md` §6) with the exact
CLI command at every step. ADR-014 records the enforcement model this
walkthrough relies on.

> **Read first.** [`docs/ONBOARDING.md`](./ONBOARDING.md) — how to
> install and adopt GKS in a project. This doc assumes that's done and
> covers what to do every day after.

> **Taxonomy v2.3 note (2026-05-13)**: prefix names below reflect v2.3 —
> `FRAMEWORK--` (architectural framework / governance, formerly `FRAME--`),
> `FRAME--` (Block Manifest, runtime entry-point of a Genesis Block,
> contract: `SPEC--GENESIS-BLOCK-MANIFEST`), `GUARD--` (renamed from
> `GUARDRAIL--`), plus new prefixes `STACK--`, `SPEC--`, `COGNITIVE--`,
> `SAFETY--`. See [`KNOWLEDGE-TYPES.md`](./KNOWLEDGE-TYPES.md) for the
> full table. The doc-to-code phases below are unchanged by v2.3; only
> the prefix vocabulary tightened up.

---

## The phases at a glance

> The P0..P6 chain below is the **Block Assembly** half of the Genesis Block Cycle; its top-down counterpart is **Block Decomposition** (12-Stage Symbol Graph DAG). See `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md` for the unified vocabulary.

```
P0 FRAMEWORK  →  P1 CONCEPT  →  P2 ADR/FEAT/ENTITY/SPEC  →  P3 BLUEPRINT  →  P4 TASK  →  P5 src/  →  P6 AUDIT
   foundation     why?          what?                       how-plan?       chunks      code         results
```

P0 is optional for incremental work — only required when introducing a new architectural framework or governance rule.

| Phase | Atom | Tier   | Storage             |
|-------|------|--------|---------------------|
| P0    | `FRAMEWORK--` · `FRAME--` (Block Manifest) | strict | `gks/framework/` · `gks/frame/` |
| P1    | `CONCEPT--` · `COGNITIVE--` | strict | `gks/concept/` · `gks/cognitive/` |
| P2    | `ADR--` · `ENTITY--` · `API--` · `FEAT--` · `SPEC--` · `STACK--` · `MOD--` | strict | `gks/{adr,entity,api,feat,spec,stack,mod}/` |
| P3    | `BLUEPRINT--` | strict | `gks/blueprint/` (YAML) |
| P4    | *(not an atom — see ADR-015)* | tracker | live task state lives at the orchestrator, not in `gks/` |
| P5    | (none — code) | —      | `src/` with `linked_symbols` citing back |
| P6    | `AUDIT--`     | strict | `gks/audit/` |

Cross-phase atoms: `GUARD--` (runtime invariants) and `SAFETY--` (ethical / alignment rules) typically sit at P0 or P2 depending on whether they're foundational or feature-scoped.

Strict-tier atoms are authored directly in dedicated branches and require human review via **Pull Requests** before merging into the canonical tree; light-tier atoms can be written directly to `main`. See ADR-012.

---

## The happy path (one feature, end to end)

Scenario: per-tenant rate-limiting on the API.

### 0. Recall before you build

The Agent Rule §6.3 starts here — check whether this decision already
exists before generating anything new.

```sh
gks recall "rate limiting per-tenant" --top-k=5
gks lookup-by-symbol src/api/rate-limit.ts          # any atom already cite this code?
```

Empty? Proceed. Hit? Read it first — you may be reinventing.

### 1. Scaffold all four atoms in one shot

```sh
gks new-feature rate-limit \
  --title="Per-tenant token-bucket rate limiting" \
  --concept="API needs per-tenant fairness — current global limiter starves small tenants" \
  --adr="Token bucket per (tenant, route); refill on schedule" \
  --blueprint-file=src/api/rate-limit.ts \
  --blueprint-file=src/db/quota.ts \
  --task=token-bucket \
  --task=middleware-wiring \
  --task-tracker=local
```

Writes 4 atom candidates directly to their canonical locations (or into a staging branch):

```
CONCEPT--RATE-LIMIT
ADR--RATE-LIMIT
FEAT--RATE-LIMIT
BLUEPRINT--RATE-LIMIT          (geography pre-filled with the two files)
```

Microtasks (`--task=…`) are **not** atoms (ADR-015) — they are
execution state owned by the orchestrator. With
`--task-tracker=local`, skeleton YAML files land in
`.brain/<ns>/tasks/rate-limit/T1_token-bucket.task.yaml` etc., outside
`gks/`. With `--task-tracker=msp` (default) or `external`, the
scaffolder prints guidance lines for the orchestrator / external
tracker to consume and writes nothing.

### 2. Review via Pull Request

Instead of a local `inbound/` queue, GKS now integrates with standard git workflows.

```sh
git checkout -b feat/rate-limit
# Author atoms (either via 'gks new-feature' or manual write)
git add gks/
git commit -m "docs: define rate-limiting knowledge chain"
git push origin feat/rate-limit
```

Review happens on the Git hosting platform (GitHub/GitLab). Once the PR is merged, the atoms are considered "promoted" and stable.

After every merge, the re-indexer rebuilds the index (e.g., `npm run msp:index`).

### 3. Verify the chain before writing code

```sh
gks verify-flow FEAT--RATE-LIMIT
# verify-flow FEAT--RATE-LIMIT
#   visited: 5 atom(s)
#   edges:   6 crosslink(s)
#   status:  OK
```

Exit 1 means a node is `draft`/missing or a link is broken. Fix the
chain *before* coding — that's the whole point of the gate.

### 4. Implement (P5)

Write the actual code in `src/api/rate-limit.ts` and `src/db/quota.ts`.
The blueprint's geography already cites these paths, so reverse lookup
works the moment the files exist:

```sh
gks lookup-by-symbol src/api/rate-limit.ts:TokenBucket
# → BLUEPRINT--RATE-LIMIT  blueprint  "Per-tenant token-bucket rate limiting"
```

### 5. Pre-push gate (drift detection)

`examples/drift-detection/pre-push-hook.sh` runs `gks lookup-by-symbol`
on every changed file. If a code path has citations but the cited
atoms haven't been touched, the push is blocked until you confirm or
update the docs.

### 6. Post-merge — write the AUDIT (P6)

After CI is green and the feature is merged:

```sh
# Author the audit atom directly in gks/audit/
# or use a template
git add gks/audit/AUDIT--RATE-LIMIT.md
git commit -m "audit: verify rate-limiting"
```

Audit body records: which acceptance criteria from `FEAT--RATE-LIMIT`
passed, perf measurements, residual risks. This closes the loop —
`crosslinks.references: [FEAT--RATE-LIMIT, BLUEPRINT--RATE-LIMIT]`
makes future readers traceable from outcome back to decision.

---

## The hotfix escape hatch (when prod is down)

Master-spec §6.4 / ADR-014. You don't have time for P1–P3 — ship
first, document within 48 h.

### 1. Tag the commit

```sh
git commit -m "HOTFIX: rate limiter overflow — emergency cap"
```

### 2. Open the hotfix atom

```sh
gks hotfix open $(git rev-parse HEAD) \
  --title="prod down: rate limiter overflow" \
  --file=src/api/rate-limit.ts \
  --reason="customer escalation"
# → HOTFIX--<7-char-sha>  valid_to = now + 48 h
```

The pre-commit gate (`examples/drift-detection/hotfix-gate.sh`) does
not block during the 48 h window. It blocks afterwards if the backfill
isn't done.

### 3. Backfill within 48 h

```sh
gks new-feature rate-limit-fix \
  --title="Rate limiter overflow root-cause fix" \
  --blueprint-file=src/api/rate-limit.ts
# review + promote as in the happy path
```

The backfill atoms must declare `crosslinks.resolves: [HOTFIX--<sha>]`
in their frontmatter. That's how `gks hotfix close` knows the debt is
paid.

### 4. Close the hotfix

```sh
gks hotfix close HOTFIX--ABC1234 \
  --resolved-by=ADR--RATE-LIMIT-FIX \
  --resolved-by=BLUEPRINT--RATE-LIMIT-FIX
```

Audit log records `hotfix_open` + `hotfix_close` with full trace.

### 5. If the 48 h window expires

`gks hotfix check --file=src/api/rate-limit.ts` exits 1. Pre-commit
blocks any further changes to the file until backfill atoms exist
*and* `gks hotfix close` has been run.

---

## Agent Rule (§6.3) as a single command

The four-step rule the master spec imposes on every agent before
writing code becomes one CLI invocation:

```sh
gks verify-flow FEAT--<NAME>
```

Returns exit-0 iff:
1. The FEAT exists in the index
2. Status is `stable` (or master-spec `APPROVED` — the alias maps it)
3. Every referenced ADR / CONCEPT / BLUEPRINT exists and is `stable`
4. No broken crosslinks anywhere in the reachable chain

Wire it into the agent harness:

```sh
gks verify-flow "$FEATURE_ID" || { echo "stop + request promotion"; exit 1; }
```

---

## Status transitions

```
raw  →  draft  →  stable  →  deprecated
                     │           │
                     └─────┬─────┘
                           ▼
                       (atom continues to exist; crosslinks stay live)
```

| From | To | When | How |
|---|---|---|---|
| (none) | `draft` | PR created | automatic / manual |
| `draft` | `stable` | PR merged | automatic (merge) |
| `stable` | `deprecated` | superseded | new ADR with `crosslinks.supersedes` |

Master-spec wording (`APPROVED`, `Accepted`) is accepted at the input
boundary — `normaliseStatus()` maps it to `stable`. See ADR-014 item 2.

---

## CI / git-hook integration

Three hooks compose into a defence-in-depth stack:

```
.git/hooks/pre-commit      ← hotfix-gate.sh (48 h backfill check)
.git/hooks/pre-push        ← pre-push-hook.sh (drift detection)
.github/workflows/*.yml    ← gks verify-flow + gks validate --links
```

Install:

```sh
cp examples/drift-detection/hotfix-gate.sh   .git/hooks/pre-commit
cp examples/drift-detection/pre-push-hook.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```

Sample CI step (any platform):

```yaml
- name: GKS chain integrity
  run: |
    npm run msp:index
    npx gks validate --links
    for feat in $(ls gks/feat/FEAT--*.md | xargs -I {} basename {} .md); do
      npx gks verify-flow "$feat"
    done
```

---

## Daily-driver cheatsheet

```sh
# Discover
gks recall "<query>" [--top-k=5] [--strategy=multi]
gks lookup <ID>
gks lookup-by-symbol src/x.ts:fn

# Author
gks new-feature <slug> --title="..." [--concept=...] [--adr=...] [--blueprint-file=...]
gks propose-inbound <ID> --title="..." --file=./body.md

# Review
# Use standard Git / Pull Request workflows
git checkout <branch>
git merge <branch>

# Enforce
gks verify-flow <ID>            # chain integrity for one root
gks validate --links            # all crosslinks across the index

# Operate
gks issue new "..." --priority=high
gks hotfix open <SHA> --title="..." --file=...
gks hotfix close HOTFIX--XXX --resolved-by=<ID>

# Maintain
npm run msp:index             # rebuild atomic_index.jsonl
gks status                      # store stats
```

---

## Where atoms live

```
gks/                       canonical atom tree (committed — durable knowledge)
├── 00_index/atomic_index.jsonl   ← regenerate with msp:index
├── concept/  adr/  feat/  entity/ api/         (P1–P2 strict)
├── blueprint/                                  (P3 strict, YAML)
├── audit/                                      (P6 strict)
├── hotfix/                                     (escape hatch, light)
└── issues/                                     (live tracker, light)

# Note: The local .brain/.../inbound/ queue is deprecated in favor of PRs.
.brain/<ns>/audit/         append-only operation log (NEVER committed)
.brain/<ns>/tasks/         microtask YAML when --task-tracker=local (ADR-015)
```

`<ns>` is the namespace from `gks.config.json` — usually `default`.

---

## Boundary reminders (what GKS won't do for you)

GKS is a storage engine (ADR-008). It does not:

- **Run timers** beyond the local pre-commit hook. The 48 h hotfix
  window is enforced *on this repo* — distributed enforcement is the
  orchestrator's job (ADR-009).
- **Verify code symbols exist.** GitNexus / your AST tool does that;
  GKS only records the citation.
- **Schedule reviewer notifications.** Add a CI bot or a Slack
  webhook on the PR merge event.
- **Decide *when* to run `verify-flow`.** That's policy — your
  pre-commit hook or CI step decides.
- **Validate atom *content* against domain rules.** Frontmatter shape
  is checked; "is this ADR a good ADR" is human work.

---

## Further reading

- [`ONBOARDING.md`](./ONBOARDING.md) — adopt GKS in an existing or new project
- [`KNOWLEDGE-TYPES.md`](./KNOWLEDGE-TYPES.md) — full atom taxonomy (35+ prefixes after v2.3)
- [`adr/014-doc-to-code-enforcement.md`](./adr/014-doc-to-code-enforcement.md) — the model behind this workflow
- [`adr/010-reverse-citation-lookup.md`](./adr/010-reverse-citation-lookup.md) — `lookup-by-symbol` semantics
- [`adr/012-extended-taxonomy.md`](./adr/012-extended-taxonomy.md) — strict vs light tier
- [`TECHNICAL-OVERVIEW.md`](./TECHNICAL-OVERVIEW.md) — internals + complete API reference
- [`MSP_RELATIONSHIP.md`](./MSP_RELATIONSHIP.md) — what an orchestrator above GKS looks like
