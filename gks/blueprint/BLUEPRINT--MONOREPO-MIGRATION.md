---
id: BLUEPRINT--MONOREPO-MIGRATION
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — monorepo migration phased implementation plan
tags:
  - monorepo
  - workspace
  - migration
  - blueprint
  - implementation
  - msp
  - gks
crosslinks: {"references":["CONCEPT--MONOREPO-MIGRATION","ADR--MONOREPO-STRUCTURE","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
linked_symbols:
  - {"file":"package.json"}
  - {"file":"packages/gks/package.json"}
  - {"file":"packages/msp/package.json"}
  - {"file":".github/workflows/ci.yml"}
  - {"file":".github/workflows/publish-gks.yml"}
  - {"file":"eslint.config.js"}
  - {"file":"upstream/gks-proposals/"}
  - {"file":"scripts/msp/propose.mjs"}
  - {"file":"packages/msp/src/memory/backlinks/"}
created_at: 2026-05-08T13:22:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: "Implementation plan"
---

# BLUEPRINT — monorepo migration

```yaml
metadata:
  title: "Monorepo migration — npm workspaces, packages/gks + packages/msp"
  parent_concept: CONCEPT--MONOREPO-MIGRATION
  parent_adr: ADR--MONOREPO-STRUCTURE

architectural_pattern: |
  Phased migration. Each phase ships independently; each phase is reversible
  by reverting its PR. We do NOT do a "big bang" migration.

  Phase A: Restructure (no behaviour change). Move src/ → packages/msp/src/,
           import GksV3 contents → packages/gks/. Wire up npm workspaces.
           Pass all existing tests. No deletions of MSP workarounds yet.

  Phase B: Boundary enforcement. Add ESLint rule + CI standalone-GKS job.
           Verify no leakage. AUDIT.

  Phase C: Absorb upstream proposals one at a time. Each proposal #01..#04
           becomes one PR that lands the GKS-side change AND deletes the
           MSP-side workaround. AUDIT per proposal.

  Phase D: Cleanup. Delete upstream/gks-proposals/ directory. Update
           CLAUDE.md two-repo sync rule → monorepo sync rule. Archive
           Freshair129/GksV3 with a redirect README. Final AUDIT.

phase_a_restructure:
  goal: "Repo layout changed; no behaviour change; existing CI green."

  steps:
    A1. Create the workspace skeleton on a new branch (claude/msp-monorepo-A-restructure):
        - Add packages/ directory
        - Move src/, test/, scripts/, web/, dist/, vitest.config.ts,
          tsconfig*.json into packages/msp/
        - Move package.json's MSP-specific fields into packages/msp/package.json
        - Top-level package.json becomes:
            {"name": "freshair129-stack", "private": true,
             "workspaces": ["packages/*"],
             "scripts": {"test": "npm test --workspaces --if-present", ...}}
        - Top-level keeps: .github/, CLAUDE.md, README.md, ROADMAP.md,
          msp_spec.md, gks/ (atom store), upstream/, .brain/

    A2. Import GksV3 source:
        - Clone Freshair129/GksV3 at HEAD (3.6.0 tag)
        - Copy its src/, test/, README.md, CHANGELOG.md, package.json,
          tsconfig*.json into packages/gks/
        - DO NOT preserve git history (acceptable per ADR; tag GksV3 as archived after)
        - Remove GksV3-specific CI files (will rebuild unified CI in step A4)

    A3. Wire workspace dep:
        - In packages/msp/package.json, change "@freshair129/gks": "^3.6.0"
          to "@freshair129/gks": "workspace:*"
        - Run `npm install` at root → verify symlink
          packages/msp/node_modules/@freshair129/gks → packages/gks/

    A4. Unified CI:
        - .github/workflows/ci.yml runs:
            * checkout
            * npm ci
            * npm run -w packages/msp typecheck
            * npm run -w packages/msp test
            * npm run -w packages/gks test
        - Matrix on Node 20 + 22 unchanged

    A5. Verify:
        - npm test passes both packages
        - npm run build at root produces packages/msp/dist/ and
          packages/gks/dist/
        - Manual: install latest msp via npm-pack on a tmp project — still works

    A6. AUDIT--MONOREPO-RESTRUCTURE atom

  blockers:
    - Path-hardcoded scripts (e.g. test/hooks/pre-push.test.ts symlinks
      node_modules/) — fix paths to be workspace-aware
    - .brain/msp/projects/evaAI/ atom store paths — currently relative to
      repo root; update to packages/msp/ if we move the store, OR keep at
      root if it stays repo-level (cf. CLAUDE.md)
      Decision: keep .brain/ at repo root (it's a runtime store, not package code)
      Decision: keep gks/ atom store at repo root (it's the doc source for
      both packages — split into packages/{gks,msp}/docs/ later if needed)

phase_b_boundary:
  goal: "GKS package proven standalone; MSP can't accidentally couple to it via deep imports."

  steps:
    B1. ESLint boundary rule:
        - eslint.config.js adds `no-restricted-imports` for packages/gks/**:
            "patterns": ["**/packages/msp/**", "../msp/**"]
        - Verify with intentional violation → CI red

    B2. Standalone-GKS CI matrix job:
        - New job standalone-gks: checkout only packages/gks/, npm install
          (treats it as if cloned alone), npm test
        - This proves packages/gks/test/ doesn't depend on MSP fixtures or
          shared root tsconfig

    B3. Publish dry-run check:
        - PR job: cd packages/gks && npm publish --dry-run
        - Asserts package.json#dependencies has no workspace-internal entries

    B4. AUDIT--MONOREPO-BOUNDARY atom

phase_c_absorb_proposals:
  goal: "Land each pending upstream/gks-proposals/ entry as a single PR. Delete the MSP-side workaround in the same PR."

  per_proposal_pattern:
    1. Implement the GKS-side change in packages/gks/src/
    2. Bump packages/gks/package.json version (patch or minor)
    3. Update packages/gks/CHANGELOG.md
    4. Replace MSP-side workaround in packages/msp/src/ with the new GKS API call
    5. Delete the workaround code + tests
    6. Mark upstream/gks-proposals/0X-*.md as 🟢 merged-internally + move to
       upstream/gks-proposals/merged/
    7. AUDIT--<topic>-INTERNAL atom

  ordered_proposals:
    C1. proposal #01 — phase: 6 acceptance in gks propose-inbound CLI
        Removes: scripts/msp/propose.mjs (entire phase-6 patching)
        Removes: test/scripts/propose.test.ts phase-6 cases (replace with gks-side test)

    C2. proposal #02 — gks verify-flow --through-superseded
        Removes: any MSP-side workaround (currently MSP just lives without it)
        Adds: usage in MSP's pre-push hook tests

    C3. proposal #03 — stable gks backlinks API
        Removes: src/memory/backlinks/ (~200 LoC + tests)
        Removes: msp:backlinks npm script
        Updates: src/memory/backlinks/cli.ts → re-export from @freshair129/gks
                 (or delete entirely; check call sites)

    C4. proposal #04 — Smart Connections + nomic compat docs
        This is a docs-only change; lands as a docs/gks/ addition

phase_d_cleanup:
  goal: "Delete obsolete proposal infrastructure; refresh dev docs."

  steps:
    D1. Delete upstream/gks-proposals/ directory entirely
        - Move historical content into a single AUDIT atom for reference

    D2. Update CLAUDE.md:
        - Replace "Two-repo sync rule" section with "Monorepo workspace rule"
          - Was: "Before claiming a GKS API exists, check node_modules/@freshair129/gks/dist/..."
          - Becomes: "Before claiming a GKS API exists, check packages/gks/src/..."
        - Update path references (src/ → packages/msp/src/, etc.)

    D3. Update ROADMAP.md:
        - Note monorepo migration as a completed milestone
        - Future milestones can land cross-package atomically

    D4. Archive Freshair129/GksV3:
        - Set repo to archived on GitHub
        - Update README.md to point to packages/gks/ in this repo

    D5. AUDIT--MONOREPO-MIGRATION-COMPLETE atom (final)

geography:
  - "package.json"                                    # ← workspace root
  - "packages/gks/package.json"                       # ← GKS publishable
  - "packages/gks/src/"                               # ← from GksV3
  - "packages/gks/test/"                              # ← from GksV3
  - "packages/gks/README.md"                          # ← standalone library doc
  - "packages/gks/CHANGELOG.md"                       # ← from GksV3
  - "packages/msp/package.json"                       # ← was repo-root package.json
  - "packages/msp/src/"                               # ← was repo-root src/
  - "packages/msp/test/"                              # ← was repo-root test/
  - "packages/msp/scripts/"                           # ← was repo-root scripts/
  - "packages/msp/web/"                               # ← was repo-root web/
  - ".github/workflows/ci.yml"                        # ← MODIFIED to use workspaces
  - ".github/workflows/publish-gks.yml"               # ← NEW
  - "eslint.config.js"                                # ← MODIFIED with boundary rule
  - "CLAUDE.md"                                       # ← MODIFIED in phase D
  - "ROADMAP.md"                                      # ← MODIFIED in phase D
  - "gks/"                                            # ← STAYS at repo root (atom store)
  - ".brain/"                                         # ← STAYS at repo root (runtime store)
  - "upstream/gks-proposals/"                         # ← DELETED in phase D

verification_plan:
  phase_a:
    - All existing MSP tests pass under new workspace layout: npm test
    - GKS package tests pass independently: npm test -w packages/gks
    - dist/ artifacts produce identical hashes to pre-migration build
    - msp-mcp-server bin still launches and lists 11 tools (smoke test in CI)
    - Pre-commit + pre-push hooks still work (test/hooks/*.test.ts)

  phase_b:
    - ESLint catches a deliberate `import from '../msp/'` in packages/gks/
    - Standalone-GKS CI matrix is green
    - npm publish --dry-run on packages/gks/ succeeds with no workspace deps

  phase_c (per proposal):
    - The MSP feature that used the workaround behaves identically (smoke test)
    - The deleted MSP code paths have no orphan imports (typecheck)
    - GKS-side test coverage for the new API is at parity with MSP-side test it replaced

  phase_d:
    - upstream/gks-proposals/ no longer exists; CLAUDE.md no longer references it
    - GksV3 GitHub repo shows archived banner
    - ROADMAP reflects current state

implementation_order:
  A1 SCAFFOLD       packages/ tree, root package.json with workspaces
  A2 IMPORT_GKS     copy GksV3 source into packages/gks/
  A3 WORKSPACE_DEP  switch MSP's @freshair129/gks to workspace:*
  A4 UNIFIED_CI     rewrite .github/workflows/ci.yml
  A5 VERIFY         green CI on Node 20 + 22 + smoke tests
  A6 AUDIT_A        AUDIT--MONOREPO-RESTRUCTURE
  B1 ESLINT_RULE    add no-restricted-imports for packages/gks/
  B2 STANDALONE_CI  add packages/gks/ standalone test job
  B3 PUBLISH_DRYRUN add publish dry-run check on PRs
  B4 AUDIT_B        AUDIT--MONOREPO-BOUNDARY
  C1 ABSORB_01      phase-6 CLI fix + delete propose.mjs hack + AUDIT
  C2 ABSORB_02      verify-flow flag + AUDIT
  C3 ABSORB_03      backlinks API + delete src/memory/backlinks/ + AUDIT
  C4 ABSORB_04      Smart Connections docs + AUDIT
  D1 DELETE_QUEUE   delete upstream/gks-proposals/
  D2 UPDATE_CLAUDE  CLAUDE.md monorepo rule
  D3 UPDATE_ROADMAP ROADMAP entry
  D4 ARCHIVE_GKSV3  GitHub repo settings + redirect README
  D5 AUDIT_FINAL    AUDIT--MONOREPO-MIGRATION-COMPLETE
```

## Implementation notes

- **Phases are independent PRs.** Phase A merges and ships before B starts. Each "ABSORB_NN" is its own PR. Bisect-friendly.
- **Worktree caveat (per `CLAUDE.md`)**: subagents running phase A in a worktree must `npm ci` after `npm install` to avoid `npx`-related test-runner flakes.
- **Path migrations are mechanical**: most `src/foo` → `packages/msp/src/foo` replacements can be done with `git mv` and a single sed pass on tsconfig paths. Avoid clever scripts; one big mechanical commit per phase A step is fine.
- **Atom store stays at root** (`gks/<type>/`) — these are docs about the project, not code in either package. Splitting into `packages/{gks,msp}/docs/` is a future bikeshed.
- **`.brain/` stays at root** — runtime memory store is a project-level artifact.
- **Don't migrate GksV3 git history.** Pretending it's a clean import keeps the repo tidy. The archived GksV3 repo retains pre-merge history for anyone who wants it.

## Implementer: do NOT do

- **Don't merge GKS into MSP's package.** It must remain separately publishable per `[[ADR--MONOREPO-STRUCTURE]]`.
- **Don't drop `@freshair129/gks` from npm.** External-consumer compat is non-negotiable in this migration.
- **Don't combine phases.** Each phase ships independently. A monolithic "do everything" PR fails review.
- **Don't refactor MSP↔GKS API surface during phase A.** Move files, don't change behaviour. API refactors happen in later milestones with their own ADRs.
- **Don't move atom stores or `.brain/` into packages.** They are project-level.

## Source

- `[[CONCEPT--MONOREPO-MIGRATION]]` — motivation
- `[[ADR--MONOREPO-STRUCTURE]]` — decided structure
- `upstream/gks-proposals/0[1234]-*.md` — proposals to absorb in phase C
- npm workspaces docs (https://docs.npmjs.com/cli/v10/using-npm/workspaces)

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

