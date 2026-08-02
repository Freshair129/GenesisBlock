# Platform-Layer Audit: GoVibe / MSP / GKS (+ NotiKeeper, engine-repo internals)

Due-diligence pass, 2026-07-07. Everything below is verified on disk; NOT FOUND is stated where applicable.

## 0. Path existence check (task item 1)

| Path | Exists? | What it is |
|---|---|---|
| `G:\govibe` | YES | Git repo (`Freshair129/govibe`), Vite/React "Mission Control" dashboard + doc-governance + MCP scripts. Has CODE: `G:\govibe\src` (39 .ts/.tsx), `packages/govibe-core`, `scripts/`, `package.json` |
| `D:\GoVibe` | YES | Turborepo scaffold ("AI-Native Visual Vibe Code Platform"), `apps/{desktop,mobile}`, `packages/{config,core,genesis-db,ui}`. **No git remote** (`git -C D:\GoVibe remote -v` → empty). Last commit 2026-06-08 — stale fork/ancestor of G:\govibe |
| `G:\G-Maiden` | YES | Active Tauri app + orchestration + studio (`Freshair129/G-Maiden`, last commit 2026-07-06) |
| `G:\NotiKeeper` | YES | Android app + `mcp-server/` (`Freshair129/notikeeper`, last commit 2026-06-30) |
| engine `mcp/` | YES | Exactly one file: `G:\GenesisBlock_Dev\GenesisBlock\mcp\server.js` (143 lines) |
| engine `.brain/gks` | YES | `storage/` containing `genesis-graph.wal` (**0 bytes**) and `identity.bin` (32 bytes) — empty DB shell, dated Jun 8 |
| engine `.agents/` | YES | Agent persona markdown + `agent-registry.yaml` + validator script — identity docs, not runnable product |
| engine `gks/` | **NOT FOUND** | Does not exist in the engine repo |
| `G:\msp` | YES but **EMPTY** | Directory contains zero files (`ls -a /g/msp` → only `.` `..`) |
| `G:\GenesisBlock_Dev\Rwang_remote\gks\` | YES | Real code (see GKS section) |

Bonus finding on install premise: the engine npm package is **not published**. `package.json` name is `@freshair129/gks-genesis-block-native` v0.2.0 (`G:\GenesisBlock_Dev\GenesisBlock\package.json`), and both `npm view genesis-block-native` and `npm view @freshair129/gks-genesis-block-native` return **E404**. A stranger cannot `npm install` the engine at all today — clone + `npm run build` (Rust toolchain + napi) is the only path.

## 1. GoVibe

Two divergent codebases sharing a name:

### G:\govibe ("GoVibe Mission Control") — the live one
- **Runnable code exists**: `package.json` (`govibe-mission-control` 0.1.0, private) with `dev`/`build`/`test`/`e2e` scripts; `src/` React app; `scripts/mcp/govibe-mcp-server.mjs` — a hand-rolled stdio JSON-RPC MCP server (`runtime-core.mjs`, `handlers.mjs`, `registry.mjs`, `sidecar-server.mjs`) with its own tests. Also `bin: {"govibe": "./packages/govibe-core/bin/init.mjs"}` — a CLI entry, but the package is `"private": true` and unpublished.
- **Install story for a stranger**: weak. **No root `README.md`** (`ls /g/govibe/README.md` → not found); top level is E2E checklists, `PRODUCT.md`, `SETUP_COMPLETE.md`, session summaries. Not published anywhere. Last commit 2026-06-22.
- **Hardcoded author paths**: `scripts/agents/README.md:43-257` — every invocation example is `powershell ... -File "G:\govibe\scripts\agents\...ps1"`; `scripts/agents/run-gemini-doc-audit.ps1:52` — prompt literally says `You are working in G:\govibe.`; `scripts/agents/record-codex-hybrid-round.ps1:101` same pattern. `engine/orchestration/logs/*.log` are committed author session transcripts full of `C:\Users\freshair\AppData\Local\Temp\...` and account/model metadata (8 files in code dirs match `G:/|C:/Users|freshair`).
- **Assumed services**: `claude` CLI, PowerShell agent scripts, local Ollama/codex (per script names), Playwright.
- **Consumed by**: nothing external found. It is the *consumer-of-docs* layer (all the MSP/GKS SDDs live here).

### D:\GoVibe — stale scaffold
- Turborepo (`package.json` name `govibe`, workspaces `apps/*`, `packages/*`), `run-govibe.bat` launcher (checks npm/cargo, runs `turbo dev`). `apps/desktop` is a Tauri shell (`src-tauri/`, React `src/`); total ~62 .ts/.tsx/.rs files across apps+packages.
- **The engine integration is a hollow stub**: `D:\GoVibe\packages\genesis-db\` contains **only** `backend-stub/README.txt`, which is a generic Thai tutorial about backend folder layout — zero engine binding, zero code. `README.txt` at repo root is likewise a Thai monorepo-vs-polyrepo tutorial, not project docs.
- No git remote, last commit 2026-06-08 ("migrate ActivityHeatmap"). Effectively abandoned in favor of G:\govibe.

**Verdict — GoVibe**: exists-as-code? *Partially* (dashboard + MCP scripts at G:\govibe; platform monorepo at D:\GoVibe is a stub). Author-coupled? YES (G:\govibe literals in every agent script, freshair transcripts committed). Product gap: **L** for Mission Control, **XL** for the "platform". No stranger-install path on either.

## 2. MSP (Memory & Soul Passport)

**Code = NOT FOUND.** Blunt version: MSP is a documentation universe with an empty directory named after it.

- `G:\msp` exists and is **empty** (zero entries).
- No repo or module implementing sessions/episodic memory/belief revision was found anywhere searched (G:\ top level, G:\GenesisBlock_Dev, G-Maiden src/orchestration/docs, D:\GoVibe).
- What exists is docs: `G:\govibe\docs\architecture\SDD-GoVibe-MSP-GKS-Integration.md`, `SDD-MSP-External-Evidence-Boundary.md`, `docs\adr\ADR-014-MSP-GKS-Traceability-Gate.md`, `docs\architecture\MSP-GKS-Taxonomy-Mapping.md`, `docs\runbooks\RUNBOOK-MSP-Validate-Evidence-Adapter.md`, `docs\features\traceability-audit\FEAT-MSP-Validate-Evidence-Adapter.md`, `docs\change-requests\CR-2026-06-14-MSP-GKS-GoVibe-Integration.md`; definition at `G:\govibe\docs\BRD-GoVibe-Platform.md:206` ("Memory & Soul Passport — Memory OS ที่เดินทางไปกับ agent"); engine-side `docs\ROUND4-ENGINE-VS-PLATFORM.md:24` positions MSP against Mem0/Letta/Zep; `G:\GenesisBlock_Dev\CR--MSP-CONTEXT-CHECKPOINTING-AND-DELEGATED-EXPLORATION.md` is a schema-level CR ("Out of scope: Runtime checkpoint implementation").
- The only file with "msp" in its name that runs: `G:\govibe\scripts\docs\msp-evidence.mjs` — a git/validation **evidence collector for doc traceability**, not a memory system. Do not mistake it for MSP runtime.

**Verdict — MSP**: exists-as-code? **NO — docs-only.** Author-coupled? n/a (nothing to couple). Product gap: **XL** (greenfield; the docs themselves say runtime is out of scope).

## 3. GKS (Genesis Knowledge System)

There is no single GKS codebase. Today "GKS" resolves to three unrelated artifacts:

1. **Engine MCP server**: `G:\GenesisBlock_Dev\GenesisBlock\mcp\server.js` — 143 lines, 3 tools (`query_hql`, `retrieve_tiered_context`, `add_knowledge`), `require("../index.js")` — i.e. it only works inside a clone where the native addon has been built locally. DB path defaults to `.brain/mcp_db` via `GENESIS_DB_PATH` (server.js:12).
2. **Engine GKS storage**: `.brain/gks/storage/` — `genesis-graph.wal` is **0 bytes**, `identity.bin` 32 bytes, untouched since Jun 8. GKS-as-data is empty; nothing writes to it.
3. **Rwang's `gks/` module**: `G:\GenesisBlock_Dev\Rwang_remote\gks\` — this is the only substantial GKS *code*: ~18 .mjs modules each with a test file (`atom-schema.mjs`, `backlog.gorch.json`, `compile.mjs`, `engine-dispatch.mjs`, `marketplace.mjs`, `entitlement.mjs`, `verify-gate.mjs`, `goldset.mjs`, `telemetry.mjs`, `approval-chain.mjs`, `a2a-surface.mjs`, ...). But it is Rwang's **task-atom backlog/marketplace subsystem**, not a knowledge database — it does not touch the engine's `.brain/gks` storage.

Peripheral: `G:\GKS_Backups` (top-level dir, data backups, not audited), `D:\gks_genesis_knowledge_system_summary (1).md` (loose note), govibe docs `docs\srs\SRS-GKS-Retrieval-Layer.md` + `ADR-017-GoVibe-Governance-Translator-GKS-Interlingua.md`. The engine's own npm name carries the brand (`@freshair129/gks-genesis-block-native`) yet is unpublished (E404).

**What would a stranger install?** Nothing. No package, no installer, no unified repo. The closest runnable thing is `npm run mcp:start` inside a cloned+built engine repo.

**Verdict — GKS**: exists-as-code? **Fragmented** — 143-line MCP shim (engine) + empty storage + a Rwang-internal atom subsystem that shares only the name. Author-coupled? YES (Rwang gks ships `backlog.gorch.json`/`goldset.data.json` author state in-tree; engine MCP requires local build). Product gap: **XL** as a coherent system.

## 4. NotiKeeper (consumer check)

- **Real shipped product**: Android app with 8 APKs committed at repo root (`NotiKeeper-v1.4.apk` … `v1.11.apk`), proper `README.md` (Thai, with build instructions "ทาง A — Android Studio"), `ARCHITECTURE.md`/`SECURITY.md`/`RELEASE.md`, GitHub remote. This is the *most* installable artifact in the whole stack — install = sideload an APK.
- **mcp-server**: `G:\NotiKeeper\mcp-server\` — node, `package.json` deps are `@modelcontextprotocol/sdk`, **`better-sqlite3`** (primary store: `graph.db`, `relations.db`), qrcode, zod. **The engine is NOT a dependency**; it is consumed via a hardcoded absolute path: `graph-index.mjs:32` → `const GENESIS_PKG = "G:/GenesisBlock_Dev/GenesisBlock/index.js"`. Breaks on any machine that isn't the author's. Further author-coupling: `scraper.mjs:22` hardcodes `C:\Users\freshair\AppData\Local\GoVibeToolchains\node-v24.16.0-win-x64`; `scrape-all.mjs:15-16` uses `G:\NotiKeeper\...` in usage docs.
- So the "first real embedded consumer of the engine" consumes it as a **side index behind SQLite**, wired by an author-machine path — evidence of platform/engine sharing being one `require()` line, not an SDK contract.

**Verdict — NotiKeeper**: exists-as-code? YES (real, current). Author-coupled? App: no; mcp-server: YES. Gap: **S** (app) / **M** (mcp-server portability).

## 5. Cross-cutting evidence on author-coupling

- Rwang worker prompts hardcode the target project: `Rwang_remote\engine.mjs:433` `- root = G:/G-Maiden` (and :408, :429 name G-Maiden explicitly) — the orchestrator is currently a G-Maiden-specific tool despite its generic README (`Rwang_remote\README.md` — the best README in the stack: "Zero external dependencies").
- `Rwang_remote` ships `accounts.local.json` in-tree (author account state alongside `accounts.example.json`).
- `G:\govibe\engine\orchestration\logs\` commits full Claude session transcripts with the author's temp paths, model usage, and cost data.

## 6. Verdict table (task item 5)

| Layer | Exists as code? | Stranger-installable? | Author-coupled? | Gap |
|---|---|---|---|---|
| Engine (context) | YES (Rust, tests, CI) | NO — npm package unpublished (E404 both names); clone+build only | Low | **M** |
| Rwang (orchestrator) | YES, tested, good README | Clone+node, plausible — but engine.mjs hardcodes G:/G-Maiden; accounts.local.json in tree | Medium | **M** |
| GoVibe (G:\govibe) | YES (dashboard + MCP scripts) | NO — no root README, private, unpublished | High (G:\govibe literals, freshair transcripts) | **L** |
| GoVibe platform (D:\GoVibe) | Scaffold only; genesis-db pkg = 1 tutorial README.txt | NO — no remote, stale since 06-08 | Medium | **XL** |
| MSP | **NO — docs-only; G:\msp is empty** | n/a | n/a | **XL** |
| GKS | Fragmented: 143-line MCP shim + 0-byte storage + Rwang-internal atoms module | NO — nothing to install | High | **XL** |
| NotiKeeper | YES — shipped APKs + SQLite mcp-server | APK yes; mcp-server no (G:/ hardcode at graph-index.mjs:32) | mcp-server: high | **S/M** |

**Engine-wedge sharing conclusion**: the only code the platform layers actually share with the engine today is (a) NotiKeeper's single hardcoded `require("G:/GenesisBlock_Dev/GenesisBlock/index.js")` and (b) the engine's own 143-line `mcp/server.js`. There is no SDK-mediated, path-independent consumption anywhere; MSP and a unified GKS exist only on paper.