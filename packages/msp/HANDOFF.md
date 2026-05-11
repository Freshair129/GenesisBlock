# HANDOFF — manual tasks remaining at v0.4.0

> **Context**: All-M plan complete. v0.4.0 shipped (Tier 1 + Tier 2 draft impl). The tasks below are things I (the AI agent who built this) **cannot do** because of environment restrictions or because they require human judgement / external systems. They are listed in priority order with copy-paste commands where possible.
>
> Read first: [`gks/audit/AUDIT--V0-4-0.md`](./gks/audit/AUDIT--V0-4-0.md), [`gks/concept/CONCEPT--TIER-3-DEFERRED.md`](./gks/concept/CONCEPT--TIER-3-DEFERRED.md), [`CLAUDE.md`](./CLAUDE.md), [`ROADMAP.md`](./ROADMAP.md).

---

## ⛳ Priority 1 — Push tag v0.4.0 (5 minutes)

**Why I can't**: The git server returned `HTTP 403` when I tried `git push origin v0.4.0` from this sandbox — the credentials issued to this session are scoped for branches, not tags.

**What you need to do**:

```bash
git checkout main
git pull origin main

# Verify the tag exists locally (created during PR #38 merge close-out)
git tag -l v0.4.0

# If missing, recreate:
git tag -a v0.4.0 -m "MSP v0.4.0 — governance mechanism complete"

# Push it:
git push origin v0.4.0
```

**Verify on GitHub**: https://github.com/Freshair129/msp/tags should now show `v0.4.0`.

**Optional**: create a release with auto-generated notes:
```bash
gh release create v0.4.0 \
  --title "MSP v0.4.0 — governance mechanism complete" \
  --notes-from-tag
```

---

## ⛳ Priority 2 — Submit 5 GKS upstream proposals (10 minutes)

**Why I can't**: My GitHub MCP tools are scoped to `Freshair129/msp` only. Calls against `Freshair129/GksV3` are blocked.

**What's ready**: 5 fully-drafted proposal files under [`upstream/gks-proposals/`](./upstream/gks-proposals/) — each with `Why / What (diff sketch) / Compat / Test plan / Atom reference`. The submission packet at [`upstream/gks-proposals/SUBMISSION.md`](./upstream/gks-proposals/SUBMISSION.md) has **copy-paste-ready GitHub issue bodies** for all 5.

**Recommended strategy** (Strategy B from `SUBMISSION.md`): open 5 separate issues against `Freshair129/GksV3`. ~2 minutes per issue.

| # | Topic | What MSP gains when it lands |
|---|---|---|
| 01 | Accept `phase: 6` in `gks propose-inbound` CLI | Removes `scripts/msp/propose.mjs`'s phase-6 hack |
| 02 | `gks verify-flow --through-superseded` flag | Cleaner supersede chain handling |
| 03 | Stable `gks backlinks` API | Removes `src/memory/backlinks/` (~200 LoC) |
| 04 | Smart Connections + nomic compatibility doc | Recipe for Memory OS implementers |
| 05 | Publish `@freshair129/gks@3.6.0` to npm | Unlocks `createNomicEmbedder()` for MSP |

**Step-by-step**:
1. Open `upstream/gks-proposals/SUBMISSION.md` in your editor
2. Pick "Strategy B — Four separate issues" section (it covers all 5; the README was written when only 4 existed)
3. Copy each issue body → paste into a new GitHub issue at `Freshair129/GksV3/issues/new`
4. After each is filed, edit the corresponding `upstream/gks-proposals/0X-*.md` and bump the status from `🟡 drafted` to `🔵 awaiting upstream review` + add the issue URL

**When upstream lands a proposal**, follow the workflow in [`upstream/gks-proposals/README.md`](./upstream/gks-proposals/README.md) "Workflow when an upstream lands":
1. Bump `package.json` GKS dep to the new version
2. Replace MSP workaround with the upstream call
3. Bump status to `🟢 merged upstream`
4. Write `AUDIT--<topic>-UPSTREAMED` atom

---

## ⛳ Priority 3 — Configure MCP for first-time use (3 minutes)

**Why this matters**: MSP is now production-ready, but until you wire the MCP server to your Claude Code / Cursor / Cline, the agent can't reach it.

**Step 1** — install:

```bash
cd /path/to/msp
npm install        # already done; this is just a sanity check
npm run build      # generates dist/ if not present
```

**Step 2** — add to MCP config (e.g. `~/.config/claude/mcp.json` or Cursor's equivalent):

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "npx",
      "args": ["msp-mcp-server", "--root=/absolute/path/to/your/project"],
      "env": {
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<paste from Obsidian Local REST API plugin>",
        "MSP_LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

**Step 3** — restart your agent host. You should now see 11 MSP tools (`msp_validate`, `msp_propose`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild`, `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set`).

**Optional plugins** for fuller experience:
- **Obsidian + Local REST API plugin** — enables `msp_recall`'s REST text-search path
- **Smart Connections plugin** (configured to use `nomic-embed-text-v1.5`) — human-side semantic browse in Obsidian

Without these, MSP works headless (filesystem fallback). See [`examples/setup/`](./examples/setup/) for setup guides.

---

## ⛳ Priority 4 — Decide PROTO promotion cadence (your call)

**Why I can't**: Promoting a PROTO from `draft` → `stable` requires real-world observation. Each PROTO needs to be watched against actual workloads to confirm it doesn't false-positive on legitimate atoms before becoming a hard gate.

**What's pending** — 8 governance PROTOs are currently `status: draft` (predicates run + emit findings, but no fail-exit):

| PROTO | Currently surfaces (in this repo) |
|---|---|
| `PROTO--PHASE-GATES` | 3 phase-6 AUDITs missing backing BLUEPRINT, 5 ADRs without CONCEPT |
| `PROTO--SCALING-LEVEL-GATE` | 1 FEAT missing ADR linkage |
| `PROTO--ALGO-PARAM-COUPLING` | vacuous pass (no ALGO/PARAM atoms yet) |
| `PROTO--AUTHORITY-ENFORCEMENT` | vacuous pass (no `.brain/msp/authority.yaml`) |
| `PROTO--VALID-UNTIL` | 0 violations (no in-frontmatter `valid_until` yet) |
| `PROTO--SUMMARY-MIN` | passes; overlaps with core rule |
| `PROTO--ADR-MONOTONIC` | passes; overlaps with core rule |
| `PROTO--EVIDENCE-FOR-DECISIONS` | passes; overlaps with core rule |

**Promotion checklist** per PROTO (when ready):

1. Inspect surfaced findings. Decide whether each is a real bug (fix the atom) or false-positive (refine the predicate).
2. Once happy with signal:
   ```bash
   # Edit the PROTO atom's frontmatter:
   # - status: draft → status: stable
   sed -i 's/status: draft/status: stable/' gks/proto/PROTO--<NAME>.md
   npm run msp:index
   npx tsx src/validator/cli.ts --all   # confirms exit code
   git checkout -b claude/msp-promote-<name>
   git commit -am "promote PROTO--<NAME> draft → stable"
   ```
3. Open PR. CI will exit-1 if any `severity: error` violation exists in the live tree, so bug-hunting must be done before promotion.

**Recommended promotion order** (least disruptive first):
1. `PROTO--SUMMARY-MIN` (overlaps with core; promoting just makes the PROTO authoritative)
2. `PROTO--ADR-MONOTONIC` (same)
3. `PROTO--EVIDENCE-FOR-DECISIONS` (same)
4. `PROTO--VALID-UNTIL` (no current violations; safe)
5. `PROTO--ALGO-PARAM-COUPLING` (vacuous; first ALGO/PARAM atom will surface real signal)
6. `PROTO--AUTHORITY-ENFORCEMENT` (after `.brain/msp/authority.yaml` is populated)
7. `PROTO--SCALING-LEVEL-GATE` (after fixing FEAT--MSP-MCP-TOOL-EXPANSION's missing ADR)
8. `PROTO--PHASE-GATES` (after fixing the 3 AUDITs / 5 ADRs OR adding `phase_override` to legitimate exceptions)

**M8f-2 cleanup** (after promoting M8f's 3 PROTOs to stable):
- Remove the duplicate run from `src/validator/index.ts` (existing core rules → unused once PROTO is authoritative)
- Optionally delete `src/validator/rules/{summary-min,adr-monotonic,evidence-for-decisions}.ts` (or keep as private helpers imported by the PROTO wrappers)

---

## 📋 Tier 3 milestones — wait for triggers

These are **explicitly deferred** — see [`gks/concept/CONCEPT--TIER-3-DEFERRED.md`](./gks/concept/CONCEPT--TIER-3-DEFERRED.md) for full rationale + revisit triggers + effort estimates.

| Milestone | Trigger | Action when triggered |
|---|---|---|
| **M9c** Cross-repo verify-flow | GKS upstream API exists (proposal 02 or new) | Spawn subagent reading `CONCEPT--MSP-ROADMAP` §3 M9c |
| **M9d** Notion migration | A team / project actively using Notion wants to migrate | Multi-day project; warrants its own milestone-prep PR |
| **M9e** Auto-ADR generator | ≥ 30 ADRs landed; reviewer-feedback loop established | LLM-creative; expect 2 weeks of prompt iteration |
| **M10a** msp-bridge plugin | Vault > 5,000 atoms or semantic latency > 500ms | Obsidian plugin scaffold + Smart Connections API |
| **M10b** Kuzu/Neo4j backend | Crosslinks > 50,000 or multi-hop on hot path | Backend adapter + migration tool |
| **M10c** RRF tuning | Real-world dissatisfaction OR labeled corpus exists | Corpus + harness + tuning pass |

Total deferred budget when unblocked: ~25 working days. None forgotten.

---

## 🧹 Optional cleanup

### Delete merged feature branches (cosmetic; ~30 seconds)

The remote currently has many `claude/msp-*` branches from this session. After v0.4.0 they're all merged into main. Cleanup:

```bash
# List remote-tracked branches that are merged
git remote prune origin
git branch -r --merged origin/main | grep "claude/msp-" | sed 's|origin/||'

# Delete them (review the list first; this is destructive)
git branch -r --merged origin/main \
  | grep "claude/msp-" \
  | sed 's|origin/||' \
  | xargs -I{} git push origin --delete {}
```

### Re-verify the live tree

```bash
npm ci
npm test
npm run typecheck
npm run msp:check-links
npx tsx src/validator/cli.ts --all
```

Expected: 535 tests pass, 159 atoms validate, 9 PROTOs run (7 pass + 2 surface real warnings, all draft → exit 0).

---

## 📞 If you get stuck

**Validator emits a real PROTO error you didn't expect?**
1. Read the offending atom's frontmatter — does it match the PROTO's contract?
2. Check `gks/proto/PROTO--<NAME>.md` body for the rule + escape hatches
3. If you genuinely want to skip: add `phase_override: { skip_blueprint: true, reason: "..." }` (M8b) or equivalent escape hatch documented in the PROTO

**A subagent worked on this; how do I trust the diffs?**
- Each milestone has an AUDIT atom under `gks/audit/AUDIT--<MILESTONE>.md` recording what shipped + impl decisions
- The master close-out is `AUDIT--V0-4-0.md` with crosslinks to every per-milestone AUDIT
- All PRs squash-merged with full commit message + test verification

**Want to spawn another subagent?**
- Read the prompt template in `CLAUDE.md` "Working with subagents"
- Run `npm ci` in worktree first (worktree caveat documented in CLAUDE.md)
- Always ship as `status: draft` for new PROTOs (gradual rollout)

---

## 🎁 Final state at v0.4.0

- **159 atoms** in `gks/`
- **535 tests** passing
- **11 MCP tools** (6 gatekeeper + 5 passport)
- **9 PROTOs** registered (1 sample + 8 governance, all draft)
- **27 AUDIT atoms**
- **5 upstream proposals** drafted
- **0 open PRs**
- **Production-ready**: passport core ✅, governance mechanism ✅

ขอบคุณที่ใช้เวลาทำงานด้วยกัน — วาง passport ของ agent อยู่ในมือ user แล้วครับ 🛂
