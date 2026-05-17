---
id: BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION
phase: 3
type: blueprint
scale_level: L2
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: BLUEPRINT — replace inbound queue with candidates layer (phased removal
  of msp_propose + propose.mjs + inbound infra)
tags: &a1
  - msp
  - inbound
  - candidates
  - migration
  - blueprint
  - implementation
crosslinks: &a2
  references:
    - CONCEPT--KNOWLEDGE-LAYERS-V2
    - ADR--AGENT-WRITE-BOUNDARIES
    - FRAMEWORK--MSP-ARCHITECTURE-V2
linked_symbols: &a3
  - file: src/mcp/tools/propose.ts
  - file: packages/msp/src/mcp/tools/candidate.ts
  - file: packages/msp/src/memory/candidates/writer.ts
  - file: scripts/msp/propose.mjs
  - file: test/scripts/propose.test.ts
  - file: test/mcp/tools/propose.test.ts
  - file: packages/msp/test/mcp/tools/candidate.test.ts
  - file: packages/msp/test/memory/candidates/writer.test.ts
  - file: web/src/components/CandidatesList.tsx
  - file: web/src/api.ts
  - file: packages/msp/src/mcp/server.ts
  - file: package.json
  - file: msp_spec.md
  - file: CLAUDE.md
created_at: 2026-05-08T17:02:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: BLUEPRINT — replace inbound queue with candidates layer (phased removal
    of msp_propose + propose.mjs + inbound infra)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-08T17:02:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: BLUEPRINT — replace inbound queue with candidates layer (phased removal
      of msp_propose + propose.mjs + inbound infra)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-08T17:02:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      domain: blueprint
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — inbound to candidates migration

```yaml
metadata:
  title: "Inbound queue → candidates layer (4-layer knowledge model)"
  parent_concept: CONCEPT--KNOWLEDGE-LAYERS-V2
  parent_adr: ADR--AGENT-WRITE-BOUNDARIES

architectural_pattern: |
  Strictly additive-then-subtractive migration. We do NOT delete inbound
  before candidates is proven; we do NOT keep both running long term.

  Phase 1 (additive):
    Add CandidateWriter + msp_candidate MCP tool + Knowledge Browser tab.
    No deletions. Both inbound and candidates work in parallel.

  Phase 2 (deprecate):
    Mark msp_propose tool deprecated in description. msp_propose handler
    delegates to candidate writer (still produces output, but in candidates/
    not inbound/). The agent-facing surface keeps working through one
    cycle of MCP client redeploy.

  Phase 3 (delete):
    Remove msp_propose tool, scripts/msp/propose.mjs, gks inbound CLI
    references, .brain/.../inbound/ directory. Update tests, CLAUDE.md,
    msp_spec.md §13.

  Phase 4 (audit + supersede atoms):
    Mark CONCEPT--INBOUND-QUEUE as superseded. Update FRAME--MSP-
    ARCHITECTURE-V2 if it mentions inbound. Final AUDIT.

phase_1_additive:
  goal: "Candidates layer functional alongside existing inbound."

  steps:
    1.1 src/memory/candidates/writer.ts
        - export class CandidateWriter
        - constructor({ root, namespace }) — defaults namespace 'evaAI'
        - method write({ type, proposed_id, title, body, rationale?, confidence? }):
            * validate proposed_id matches /^(CONCEPT|ADR|FEAT|BLUEPRINT|FRAME|AUDIT|PROTO)--[A-Z0-9-]+$/
            * compute path: <root>/.brain/msp/projects/<ns>/candidates/<proposed_id>.md
            * mkdir -p the directory
            * compose frontmatter:
                proposed_id: <id>
                type: <type>
                status: candidate
                proposed_at: <ISO>
                proposed_by: agent  # or 'human' if called from CLI
                rationale: <if provided>
                confidence: <if provided>
            * write file (overwrite if exists; emit warning in result)
        - method list(): readdir + parse frontmatter → array of summaries
        - method read(proposed_id): readFile + return parsed
        - method delete(proposed_id): rm file
        - NO promote method — promotion is a human PR action, not a CLI

    1.2 src/mcp/tools/candidate.ts
        - export name = 'msp_candidate'
        - export description = 'Record a structurally-shaped candidate atom...'
        - export inputSchema = { type, proposed_id, title, body, rationale?, confidence? }
        - export handler(ctx) → async (args) => {
            const writer = new CandidateWriter({ root: ctx.root })
            const result = await writer.write(args)
            return jsonResult({ candidate_path: result.path, overwritten: result.overwritten })
          }

    1.3 src/mcp/server.ts
        - register msp_candidate alongside existing 11 tools (now 12)

    1.4 web/src/api.ts
        - new endpoint client method listCandidates() / readCandidate() / deleteCandidate()
        - candidates served from a new dev-server route (mirrors atomic_index pattern)

    1.5 web/src/components/CandidatesList.tsx
        - tab in main UI showing all candidates
        - per-row: type chip, proposed_id, title (first body heading), proposed_at
        - actions: "Open" (preview), "Copy markdown" (clipboard), "Delete"
        - explicit note: "Promote a candidate by copying its markdown into
                          gks/<type>/ and opening a PR. CI will validate."

    1.6 Tests
        - packages/msp/test/memory/candidates/writer.test.ts (~10 tests):
            * happy path: writes file with right frontmatter
            * id validation rejects bad patterns
            * overwrite emits flag
            * list returns sorted summaries
            * delete removes file
        - packages/msp/test/mcp/tools/candidate.test.ts (~5 tests):
            * registers under name msp_candidate
            * happy path with tmpdir root
            * rejects malformed id
            * confidence/rationale propagated to frontmatter
            * isolated from real .brain (use tmpdir as ctx.root)

    1.7 No spec changes yet — coexistence phase. msp_spec.md §13 still mentions inbound.

  verification:
    - npm test: existing 546 + ~15 new = ~561, all pass
    - msp_candidate visible in tools/list (12 tools)
    - manual: call msp_candidate via MCP, see file in .brain/.../candidates/
    - manual: open Knowledge Browser, candidates tab lists the file

phase_2_deprecate:
  goal: "Inbound proposals delegate through candidate path. Single source of truth."

  steps:
    2.1 src/mcp/tools/propose.ts
        - update description: prefix with "[deprecated — use msp_candidate instead]"
        - handler internally constructs CandidateWriter and writes to candidates/
          instead of spawning the wrapper script
        - inputSchema unchanged (backward compatible)
        - Note: this means scripts/msp/propose.mjs is still on disk but no
          longer invoked by the MCP tool — it remains usable from CLI for
          one cycle to give external scripts time to migrate

    2.2 ROADMAP.md and CLAUDE.md
        - Add "Deprecation notice — msp_propose / inbound queue" section
        - Pointer to msp_candidate as replacement

    2.3 No file deletions. No test deletions. Existing tests for propose
        continue to pass because the handler still produces a valid result;
        the resulting file just lives in candidates/ now. Update those tests
        to assert candidates/ path.

  verification:
    - test/mcp/tools/propose.test.ts asserts result.candidate_path under candidates/
    - test/scripts/propose.test.ts (CLI wrapper) still passes (CLI route untouched in this phase)
    - manual: existing MCP clients calling msp_propose see same shape result, but file lands in candidates/

phase_3_delete:
  goal: "Single agent intake path: msp_candidate. No inbound infrastructure."

  steps:
    3.1 Delete files:
        - src/mcp/tools/propose.ts
        - test/mcp/tools/propose.test.ts
        - scripts/msp/propose.mjs
        - test/scripts/propose.test.ts

    3.2 Update src/mcp/server.ts:
        - remove propose import + registration; back to 11 + 1 (candidate) = 12

    3.3 Update package.json:
        - remove scripts.msp:propose
        - remove scripts.msp:list (gks inbound list)
        - remove scripts.msp:promote (gks inbound promote)
        - keep gks-named scripts that don't touch inbound (validate, check-links, verify, hotfix, etc.)

    3.4 Update src/validator/cli.ts:
        - --all walks gks/ ONLY; remove findProjectInbound() + the second dirs entry
        - update CLI help text accordingly

    3.5 Update test/mcp/server.test.ts + test/mcp/bin.test.ts:
        - assert tool list contains 'msp_candidate', does not contain 'msp_propose'
        - tool count = 11 (recall, remember, compress, identity-get/-set, session-append, episode-append, backlinks-rebuild, validate, run-task, candidate)

    3.6 Update msp_spec.md:
        - §11.2 — remove `review-state.mjs` reference
        - §13 Authority Matrix — replace with the simplified version from
          ADR--AGENT-WRITE-BOUNDARIES
        - any §3 / §4 mentions of inbound — rewrite to reference candidates/

    3.7 Update CLAUDE.md:
        - "Useful commands" — drop msp:propose / msp:list / msp:promote
        - "Out of scope" or "Authority Matrix" sections — rewrite per ADR

    3.8 Empty .brain/msp/projects/<ns>/inbound/ at runtime is harmless.
        Delete it if present (best-effort). The path is gitignored already.

  verification:
    - npm test: all green, no propose-test references remain
    - npx tsx src/validator/cli.ts --all: exit 0 on clean repo
    - grep -r "inbound" src/ test/ scripts/ → only refs in atoms (acceptable)
    - tools/list returns 11 tools, msp_candidate present, msp_propose absent

phase_4_audit_supersede:
  goal: "Atoms reflect the new model; nothing in canon points at deprecated infra."

  steps:
    4.1 Mark CONCEPT--INBOUND-QUEUE as superseded:
        - frontmatter: status: superseded
        - crosslinks.superseded_by: ["CONCEPT--KNOWLEDGE-LAYERS-V2"]
        - body: prepend a "Superseded by …" notice block

    4.2 Audit FRAMEWORK--MSP-ARCHITECTURE-V2:
        - if it mentions inbound, update text to reference 4-layer model
        - if it's purely structural and doesn't mention inbound, no change

    4.3 Audit any AUDIT atom that referenced inbound history:
        - leave historical AUDITs as-is (audits are immutable history)
        - they retain "inbound" mentions; that's accurate as historical record

    4.4 AUDIT--INBOUND-REMOVED atom:
        - records what shipped, what's deleted, current state
        - confirms tools/list, validator scope, spec sections

  verification:
    - npm run msp:check-links → all crosslinks resolve
    - npm run msp:index → atomic_index.jsonl reflects new statuses
    - grep "inbound" gks/ → only superseded CONCEPT or audit history mentions

geography:
  - "packages/msp/src/memory/candidates/writer.ts"                            # NEW
  - "packages/msp/src/mcp/tools/candidate.ts"                                 # NEW
  - "packages/msp/test/memory/candidates/writer.test.ts"                      # NEW
  - "packages/msp/test/mcp/tools/candidate.test.ts"                           # NEW
  - "web/src/components/CandidatesList.tsx"                      # NEW
  - "web/src/api.ts"                                             # MODIFIED — add candidate endpoints
  - "packages/msp/src/mcp/server.ts"                                          # MODIFIED phase 1 + phase 3
  - "src/mcp/tools/propose.ts"                                   # MODIFIED phase 2, DELETED phase 3
  - "test/mcp/tools/propose.test.ts"                             # MODIFIED phase 2, DELETED phase 3
  - "scripts/msp/propose.mjs"                                    # DELETED phase 3
  - "test/scripts/propose.test.ts"                               # DELETED phase 3
  - "package.json"                                               # MODIFIED phase 3 (drop msp:propose / msp:list / msp:promote)
  - "packages/msp/src/validator/cli.ts"                                       # MODIFIED phase 3 (drop inbound walk)
  - "packages/msp/test/mcp/server.test.ts"                                    # MODIFIED phase 3 (asserts new tool list)
  - "packages/msp/test/mcp/bin.test.ts"                                       # MODIFIED phase 3
  - "msp_spec.md"                                                # MODIFIED phase 3
  - "CLAUDE.md"                                                  # MODIFIED phase 3
  - "ROADMAP.md"                                                 # MODIFIED phase 2 (deprecation), phase 3 (final)
  - "gks/concept/CONCEPT--INBOUND-QUEUE.md"                      # MODIFIED phase 4 — status: superseded
  - "gks/frame/FRAMEWORK--MSP-ARCHITECTURE-V2.md"                    # MODIFIED phase 4 if needed
  - "gks/audit/AUDIT--INBOUND-REMOVED.md"                        # NEW phase 4

verification_plan:
  phase_1:
    - vitest: 15 new tests pass (writer + tool)
    - vitest: existing 546 still pass
    - tool list = 12 (existing 11 + msp_candidate)
    - manual smoke: msp_candidate writes to .brain/.../candidates/
    - Knowledge Browser shows candidates tab

  phase_2:
    - existing propose tests asserts result path now under candidates/
    - MCP clients calling msp_propose see backward-compatible result
    - no race condition (no writes to scanned dirs)

  phase_3:
    - npm test green; total test count drops by ~5–10 (propose tests removed)
    - tools/list returns 11; msp_propose absent
    - validator --all exits 0; no inbound walk
    - msp_spec.md and CLAUDE.md no longer reference inbound CLI
    - grep -r 'inbound' src/ → zero hits in code paths

  phase_4:
    - msp:check-links OK
    - CONCEPT--INBOUND-QUEUE marked superseded
    - AUDIT--INBOUND-REMOVED present and validates

implementation_order:
  P1.1 CANDIDATE_WRITER       src/memory/candidates/writer.ts + 10 tests
  P1.2 CANDIDATE_MCP_TOOL     src/mcp/tools/candidate.ts + 5 tests
  P1.3 SERVER_REG_ADD         src/mcp/server.ts +1 tool
  P1.4 BROWSER_TAB            web/api.ts + CandidatesList.tsx
  P1.5 PHASE_1_AUDIT          AUDIT--CANDIDATES-LAYER

  P2.1 PROPOSE_DELEGATE       src/mcp/tools/propose.ts → calls CandidateWriter
  P2.2 PROPOSE_TESTS_UPDATE   assert candidates/ path
  P2.3 DEPRECATION_NOTICE     CLAUDE.md + ROADMAP.md
  P2.4 PHASE_2_AUDIT          AUDIT--PROPOSE-DEPRECATED

  P3.1 DELETE_PROPOSE_FILES   propose.ts, scripts/msp/propose.mjs, related tests
  P3.2 PACKAGE_JSON_TRIM      drop 3 npm scripts
  P3.3 VALIDATOR_SHRINK       cli.ts: drop inbound walk
  P3.4 SERVER_REG_REMOVE      drop propose registration; tools = 11
  P3.5 SPEC_UPDATE            msp_spec.md §11.2 + §13 + any §3/§4 inbound mentions
  P3.6 CLAUDE_MD_UPDATE       drop msp:propose mentions, replace authority matrix
  P3.7 PHASE_3_AUDIT          AUDIT--INBOUND-INFRA-DELETED

  P4.1 SUPERSEDE_ATOM         CONCEPT--INBOUND-QUEUE → status: superseded
  P4.2 FRAME_AUDIT            FRAMEWORK--MSP-ARCHITECTURE-V2 if needed
  P4.3 FINAL_AUDIT            AUDIT--INBOUND-REMOVED
```

## Implementation notes

- **Each phase = its own PR.** Phase 1 must merge before Phase 2 starts. Bisect-friendly.
- **Phase 1 is purely additive** — easy to land first, low review risk.
- **Phase 2 is the riskiest** because it changes `msp_propose` behaviour while keeping its name. External MCP clients calling that tool will see results in a different directory. Document clearly in the PR. The deprecation period gives external scripts time to switch to `msp_candidate`.
- **Phase 3 deletes files** — irreversible without revert. Verify Phase 1 + Phase 2 have shipped and stabilised first.
- **Phase 4 is doc-only** — atom status updates and the final audit.
- **PR #41 (propose.mjs cwd fix) becomes moot once Phase 3 lands.** Recommend: close PR #41 with a comment pointing to this BLUEPRINT once Phase 1 is merged. Until then, keep #41 open as the short-term fix.
- **Worktree caveat (CLAUDE.md)** — subagents executing any phase must `npm ci` in the worktree.
- **Race condition note** — Phase 1 candidate tests must use tmpdirs (per the lesson from PR #41/#42) to avoid touching real `.brain/` and racing with anything else.

## Implementer: do NOT do

- **Don't delete `msp_propose` in Phase 1.** It must coexist for one deprecation cycle.
- **Don't combine phases into a single PR.** Each phase ships independently.
- **Don't add a `gks candidate promote` CLI.** Promotion is a human PR action, period. Adding a promote CLI rebuilds the inbound problem with new vocabulary.
- **Don't change MCP tool count assertions before the corresponding phase.** Phase 1 says 12; Phase 3 drops back to 11.
- **Don't migrate existing inbound files.** Any leftover atoms in `.brain/.../inbound/` from old runs are user-private; the user can move or discard them. No automated migration.
- **Don't modify gitignore.** `.brain/` is already ignored; candidates/ inherits that.

## Source

- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` — model
- `[[ADR--AGENT-WRITE-BOUNDARIES]]` — boundary
- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` — base architecture
- PR #41 + PR #42 CI failures — race condition that this BLUEPRINT eliminates structurally
- Existing `src/memory/episodic/writer.ts` — pattern for CandidateWriter API shape
- Existing `src/mcp/tools/episode-append.ts` — pattern for `msp_candidate` handler shape
