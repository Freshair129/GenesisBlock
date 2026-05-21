---
id: CONCEPT--EPISODE-RETENTION
phase: 1
type: concept
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Episode retention — bounded growth of <root>/gks/episode/ via age-based
  GC with error preservation
tags: &a1
  - msp
  - phase-f4
  - agents
  - episode
  - retention
  - gc
crosslinks: &a2
  references:
    - SPEC--EPISODE-ATOM
    - ADR--EPISODE-GC-POLICY
    - ADR--BRAIN-PATH-RESOLUTION
created_at: 2026-05-14T05:00:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--EPISODE-RETENTION
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Episode retention — bounded growth of <root>/gks/episode/ via age-based
    GC with error preservation
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T05:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--EPISODE-RETENTION
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Episode retention — bounded growth of <root>/gks/episode/ via age-based
      GC with error preservation
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T05:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--EPISODE-RETENTION
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: Episode retention — bounded growth of <root>/gks/episode/ via age-based
        GC with error preservation
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T05:00:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Episode retention

## Why this atom exists

`packages/msp/src/agents/result-recorder.ts` writes one markdown atom per `dispatch()` call to `<root>/gks/episode/[[EPISODE--AGENT-RUN]]-<isoTimestamp>.md`. Per `[[SPEC--EPISODE-ATOM]]` §5, episodes are **immutable** — once written, never edited. They are also write-only from the dispatcher's perspective: nothing in the runtime trims them.

For a busy agent (hundreds to thousands of dispatches per week), the episode directory grows linearly without bound. On developer laptops the cost is small; on long-lived CI runners and dedicated agent VMs, the directory can reach hundreds of MB within months, slowing globbing (`msp:index`), validation passes, and IDE scans.

This atom defines the **retention policy**: which episodes to keep forever, which to archive, and which (optionally) to delete. The companion `[[ADR--EPISODE-GC-POLICY]]` locks the *default* numeric thresholds. The implementation lives at `packages/msp/src/agents/episode-gc.ts` with a CLI wrapper at `packages/msp/src/agents/episode-gc-cli.ts` (`msp-episode-gc`).

## Goals

1. **Bounded steady-state size** — episode directory size is a function of (a) error rate and (b) a configurable retention window, not total dispatch count.
2. **Preserve the audit trail of failures** — failed/critical dispatches MUST never be silently purged. They are the most useful runtime evidence (debugging, post-mortems, contradiction discovery via the Meta-Learning Loop).
3. **Conservative by default** — destructive operations require explicit opt-in. The default path is *archive*, not delete. The CLI defaults to dry-run; `--apply` is required to mutate.
4. **No dispatcher coupling** — GC is out-of-band. `result-recorder.ts` and `dispatch.ts` are untouched. GC can be triggered by cron, by `npm run` scripts, by a developer, or by a future "consolidator" loop.

## Default policy

`gcEpisodes(root, opts?)` evaluates each `EPISODE--*.md` under `<root>/gks/episode/` (top-level only — `_archive/` is skipped):

```
keep_days = opts.keep_days ?? 30          # cutoff in days, age relative to now
delete   = opts.delete    ?? false        # if true, hard-delete; else archive
dry_run  = opts.dry_run   ?? false        # if true, plan only — touch nothing
```

For each episode:

| `created_at` ≥ `now − keep_days` | Inferred `ok` | `severity` | Action |
|---|---|---|---|
| true (recent) | * | * | **keep** |
| false (old) | `false` | * | **keep** (error retention) |
| false (old) | `true` | `critical` | **keep** (critical-as-evidence) |
| false (old) | `true` | non-critical | **archive** (or delete if `delete=true`) |

Archive target: `<root>/gks/episode/_archive/<YYYY-MM>/<original-filename>.md` where `YYYY-MM` is derived from the episode's `created_at`, not from now. This makes archive directories naturally bounded (one per month) and easy to bulk-delete or upload later.

## Inferring `ok` from episode frontmatter

`[[SPEC--EPISODE-ATOM]]` §3 does **not** mandate a first-class `ok` field, and the current `result-recorder.ts` does not write one. The GC must infer it. Looking at what *is* written:

- **Frontmatter**: `tier_used` (in `tags`), `status: stable`, optional `cost_usd`, optional `escalated_from`. No `exit_code`, no `ok`.
- **Body**: `## Output` fenced code block. Empty/whitespace-only output suggests the tier produced nothing.
- **Body bullets**: `- tier_used: T2` etc.

Inference rule (conservative — when in doubt, treat as `ok = false` so we err on the side of *keeping* the atom):

1. If frontmatter contains a literal `ok: false`, **ok = false**.
2. Else if frontmatter contains `exit_code: <n>` and `n !== 0`, **ok = false**.
3. Else parse the body for `- tier_used: <X>`. If missing, **ok = false** (malformed episode — keep it for inspection).
4. Else look for `## Output\n\n```` followed by non-empty content before the closing fence. If output is empty/whitespace-only, **ok = false**.
5. Otherwise **ok = true**.

Inference rule for `severity`:

1. Read frontmatter `severity:` if present (future-proofing — currently not written).
2. Else parse the body for `- task.severity: <X>`. Accept `critical`, `regular`, `low`.
3. Default to `regular` when missing (so old non-critical episodes are eligible for archival).

Both rules are deliberately tolerant of frontmatter-format drift. The recorder's exact frontmatter contract may evolve; GC remains defensive.

## What gets archived, not deleted

The default behaviour is *archive* because:

- Episodes are **evidence**, not noise. Moving them out of the hot scan path is cheap; losing them is irreversible.
- Future work — `[[SPEC--META-LEARNING-LOOP]]` reverse path — may want to mine archived episodes offline.
- A developer can `rm -rf gks/episode/_archive/` at any time. The reverse (recovering a deleted episode) is impossible.

`--delete` flips this to true-deletion, which is appropriate for:

- High-volume agents where archive disk cost ≈ delete cost.
- Sensitive prompts where archival is itself a leak risk (cross-reference `[[SPEC--EPISODE-ATOM]]` §9 "Encryption at rest").
- One-shot cleanup of historical bloat.

## CLI surface

`msp-episode-gc` (bin entry in `packages/msp/package.json`):

```
msp-episode-gc [--keep-days=N] [--delete] [--dry-run] [--apply] [--root=PATH] [--json]
```

Defaults are conservative:

- No `--apply` → plan only, print report, exit 0. (Implicit dry-run.)
- `--apply` + no `--delete` → archive eligible episodes.
- `--apply --delete` → permanently remove eligible episodes.
- `--json` → emit the `GcReport` shape as JSON on stdout.

The "implicit dry-run unless `--apply`" pattern matches the principle in `[[CONCEPT--MASTER-PROMOTION]]` and is the standard MSP idiom for destructive maintenance commands.

## Out of scope

- **Re-running** older episodes (replay). The format is *evidence*, not a reproducible task spec.
- **Compressing** the archive into tarballs. Filesystem-native directory is enough for now; the developer's existing tooling (`tar`, `7z`, S3 sync) handles bulk transfer.
- **Migrating** to `~/.brain/episodic/` per `[[ADR--BRAIN-PATH-RESOLUTION]]` §Open question. GC operates on whichever location the recorder currently writes to (`<root>/gks/episode/`); when the recorder moves, the GC can be re-pointed via `--root` or extended.
- **Encryption at rest**. Tracked in `[[SPEC--EPISODE-ATOM]]` §9.

## See also

- `[[ADR--EPISODE-GC-POLICY]]` — the decision atom locking the numeric defaults + archive layout.
- `[[AUDIT--PHASE-F4-EPISODE-GC]]` — what shipped under this PR.
- `[[SPEC--EPISODE-ATOM]]` — the episode atom contract this GC operates on.
