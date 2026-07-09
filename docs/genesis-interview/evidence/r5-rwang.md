# Audit: G:\GenesisBlock_Dev\Rwang_remote (Rwang / RWANG orchestrator)

Clone of https://github.com/Freshair129/RWANG.git (git remote -v), 233 tracked files, main tip `164cf7b`. Verdict up front: **"clone-and-run for the dashboard; author-machine-only for actual agent dispatch on a real project."** The scaffolding (engine, server, Studio UI, tests) is genuinely portable Node-builtins code, but the prompts, context docs, agent cwd, and knowledge store all still assume the author's disk layout and the G-Maiden parent project.

## 1. Install story

**README** — `G:\GenesisBlock_Dev\Rwang_remote\README.md` is real and above-average: prerequisites (Node ≥18, pnpm, `claude` CLI; optional Rust+Cargo, Ollama) at lines 37-43, quick-start commands at 47-56 (`GORCH_BACKLOG=gks/backlog.gorch.json node server.mjs` + `cd studio && pnpm dev`), CLI reference at 72-88, REST API at 206-218, config reference at 222-233. It honestly states origin ("extracted from G-Maiden... bring your own backlog", README.md:256-260). What it does NOT tell a stranger: that dispatch prompts are still G-Maiden-branded, that the knowledge store points at a private disk path, or that agents execute in the repo's *parent* directory (see §2).

**package.json** (`package.json:1-38`): name `rwang` v0.1.0, `private: false`, MIT, but **no `bin` field** — not installable as a CLI; only `node orchestrator.mjs`/`node server.mjs` scripts. Runtime deps: **zero** (Node built-ins only — README.md:7 claim verified; only devDependency is `@tauri-apps/cli`). **Not published to npm**: `npm view rwang` → E404. So: **GitHub-clone-only.**

**Installer/launcher**: `dev.bat` is a real first-run launcher with prerequisite checks (pnpm/cargo/node `where` checks, auto `pnpm install`) — Windows-only. Studio UI is a separate `studio/package.json` (React/Vite/@xyflow, `pnpm install` required). Tauri shell in `src-tauri/` needs Rust toolchain. No `.env.example`; secrets pattern is `accounts.local.json` (gitignored, `.gitignore:36`; a local one exists on disk but is untracked — verified via `git ls-files`). Runtime state (`state.json`, `usage.jsonl`, `logs/`, `brain/*.gdb`) is correctly gitignored (`.gitignore:5-16`).

## 2. Hardcoded coupling (the real findings)

**engine.mjs — the reported ~390-430 coupling is confirmed, and it's prompt-level, not path-level:**
- `engine.mjs:408` — every dispatched worker prompt opens with `คุณคือ worker agent ของโปรเจกต์ G-Maiden` ("you are a worker agent of the G-Maiden project"). Every agent, on every task, in any clone, is told it's working on G-Maiden.
- `engine.mjs:429-433` — the ollama full-agent hint block hardcodes: `## โครงสร้างโปรเจกต์ (G-Maiden)`, stack "Tauri v2, Axum, SQLite", "no Python files", and **`root = G:/G-Maiden`**. A local model with tools will literally be pointed at a directory that only exists on the author's machine.
- `engine.mjs:14` + `providers.mjs:296-297` — `ROOT = resolve(__dir, "..")` and Claude subprocesses spawn with `cwd: paths.ROOT`. **Dispatched agents run in the parent directory of the clone.** This is a fossil of Rwang living inside G-Maiden; for a stranger who clones to `~/RWANG`, agents get shell/file access to `~`, not the repo. Same `paths.ROOT` cwd for codex (`providers.mjs:564`) and antigravity (`providers.mjs:724`), and the ollama tool loop runs bash in that root (`providers.mjs:468-469`).
- `config.json:408-437` (`docsForContext`) and `config.json:438-519` (`scope.byPhase`) reference `docs/architecture/tech-stack.md`, `docs/architecture/engineering-spec.md`, etc. — **none of these exist in this repo** (verified: no `docs/architecture/` directory). They are G-Maiden docs; every scoped prompt cites context files a stranger doesn't have.

**Absolute author paths:**
- `config.json:524` — `store.genesisdb.bindingPath: "G:/GenesisBlock_Dev/GenesisBlock/index.js"` and `store.knowledge: "genesisdb"` (config.json:522) — the **default shipped config** points the knowledge store at the author's GenesisBlockDB checkout. Same literal default in `store/knowledge.mjs:29`, `store/genesis-sidecar.mjs:18`, and all four PoCs (`poc/genesis-roundtrip.mjs:19`, `poc/genesis-sidecar-smoke.mjs:12`, `poc/l0-smoke.mjs:18`, `poc/l1-smoke.mjs:15`, `poc/l2-smoke.mjs:15`).
- `vram-mode.mjs:24` — `OLLAMA_APP = "C:\\Users\\freshair\\AppData\\Local\\Programs\\Ollama\\ollama app.exe"` (literal user profile path; the script also `taskkill`s ollama by exe name, vram-mode.mjs:53).
- `providers.mjs:607,668-669` — OpenRouter requests send `HTTP-Referer: https://github.com/Freshair129/G-Maiden`, `X-Title: "G-Maiden Orchestrator"`.
- Branding fossils: `orchestrator.mjs:3,96`, `server.mjs:3,154` still print "G-Maiden Orchestrator".

**Env vars assumed:** `GORCH_BACKLOG` (engine.mjs:17-19 — optional, alternate backlog with isolated state), `ANTHROPIC_API_KEY` (engine.mjs:139, providers.mjs:209 — only in `apikey` auth mode; default mode is `plan` per config.json:44), `OPENROUTER_API_KEY` (accounts.mjs:217), `OPENAI_API_KEY` (codex/image, config.json:130,196), `ANTIGRAVITY_TOKEN` (config.json:177), `PLAN_MODEL` (planner.mjs:148), `RWANG_DASH_PORT/RWANG_DASH_OPEN` (.claude/hooks/open-dashboard.mjs:24-26), `CLAUDE_CONFIG_DIR`/`CODEX_HOME` for account rotation (config.json:35,113).

**Ports/services assumed:** engine HTTP on **:4577** (server.mjs:32), Studio Vite on **:5599** proxying `/api` → :4577 (studio/vite.config.ts:11-12), Tauri devUrl :5599 (src-tauri/tauri.conf.json:8), Ollama at `http://127.0.0.1:11434` (config.json:62,528), optional A1111 image at :7860 (config.json:221). Notably it deliberately does **not** use :3000 (GenesisDB REST) — "no port opened, no :3000 clash" (store/genesis-sidecar.mjs:4).

**Thai-language prompts/UI**: dispatch prompts, escalation rules, and console strings are Thai throughout (engine.mjs:408-444, providers.mjs:376, server.mjs:154) — fine for the author, a product-localization decision for anyone else.

## 3. Provider assumptions

Wired in `providers.mjs` + `config.json:4-255`, all `enabled: true` by default except image providers:
- **claude** — subprocess `claude -p --output-format stream-json --verbose` (config.json:15-21), permission modes map to `--permission-mode acceptEdits|bypassPermissions` (config.json:23-33). Health = `spawn(claude --version)` (providers.mjs:139-141).
- **ollama** — HTTP :11434, `tools: true` with a 20-iter OpenAI-style tool loop; literal model names in role chains (config.json:262-324): `qwen3.5:4b`, `gemma4:latest`, `gemma4-rust-coder:latest`, `hf.co/unsloth/gemma-4-12b-it-GGUF:UD-Q4_K_XL`, `hf.co/sillykiwi/Aroow-Rust-Coder-9B-Q4_K_S-GGUF:Q4_K_S`; embeddings pinned to `bge-m3:latest`/1024-dim (config.json:527, genesis-sidecar.mjs:19-20). Config comments are tuned to the author's exact GPU ("12GB GPU... Chrome open", config.json:68,95).
- **codex** (`codex` CLI), **antigravity** (`agy --headless`), **openrouter** (HTTP, `:free` model slugs with a "VERIFY slugs exist" warning to self, config.json:158).
- **With nothing installed:** planning/CLI/board still work — `orchestrator.mjs status/next/graph/run` (dry-run) and the :4577 server are pure Node file ops; health checks fail soft (spawn error → unhealthy, providers.mjs:325). Role resolution walks `preferred` chains with failover, and the knowledge store degrades genesisdb→file→lexical explicitly (store/knowledge.mjs:14-18,296-313). Real dispatch (`run --execute`) produces per-task failures, not a crash. This degrade-gracefully discipline is the most product-like part of the codebase.

## 4. State / storage, and GenesisBlockDB contact

Persists as flat files next to the code: `backlog.json` + `state.json` + `.state.lock` (engine.mjs:17-25; `GORCH_BACKLOG` variant keeps sibling `.state.json`), `logs/` per-agent output, `usage.jsonl` cost meter, `brain/failures.jsonl` + `brain/traces.jsonl` (file-mode knowledge), `store/.accounts-state.json`, `store/assets` (image out).

**Yes, it touches GenesisBlockDB today — via N-API, not REST:**
- `store/knowledge.mjs:125-276` — the genesisdb knowledge adapter: `createRequire(...)(bindingPath)` → `GenesisDatabase.open({path: brain/orch.gdb, vectorDim: 1024})`, then `addNode` (task + failure nodes with bge-m3 embeddings), `addEdge` (`failed_with`, `traces`), `hybridSearch` (anti-error-loop past-mistakes retrieval), and `retrieveContext` (GRL tiered grounding, knowledge.mjs:206-223). Active by default (`config.json:522` `"knowledge": "genesisdb"`).
- `store/genesis-sidecar.mjs` — a standalone, schema-pinned (`PINNED_SCHEMA_VERSION = 1`, line 17) sidecar wrapper exposing `{addNode, hybridSearch, retrieveContext, close}` — **this file plus knowledge.mjs IS the embryonic engine-wedge driver**: in-process binding, Ollama embedding shim, schema gate, graceful degrade. It is exactly the shape a Graphiti/LangGraph-style driver needs, and it's currently trapped inside the orchestrator repo with a `G:/` default path and win32-only binding assumption (knowledge.mjs:12,297).
- No REST usage: no `:3000` references except the comment disclaiming it (genesis-sidecar.mjs:4).

## 5. Verdict — gap list "my tooling → product"

Overall grade: **between "works only on the author machine" and "clone-and-run with env setup."** Precisely: *dashboard + dry-run planning are clone-and-run today; executing real agents on a stranger's project is author-machine-only* because prompts, context docs, agent cwd, and the knowledge store all still encode G-Maiden / G: drive.

| # | Gap | Evidence | Effort |
|---|---|---|---|
| 1 | Agents run in repo *parent* dir (`ROOT = ..`) — wrong/dangerous for any clone; needs a `--project <dir>` concept | engine.mjs:14, providers.mjs:296-297 | **S** (mechanical) but **M** to do right (project-target config + docs) |
| 2 | G-Maiden hardcoded in every dispatch prompt incl. `root = G:/G-Maiden` | engine.mjs:408,429-433 | **S-M** — templatize prompt from a project manifest |
| 3 | `scope`/`docsForContext` reference docs that don't exist in the repo | config.json:408-519 | **M** — per-project scope config or auto-discovery; core to the "bring your own backlog" promise |
| 4 | Knowledge store default = author's GenesisBlockDB checkout path; binding win32-only | config.json:524, knowledge.mjs:29, genesis-sidecar.mjs:18 | **M** — depends on GenesisBlockDB shipping as an npm dep (`genesis-block-native` prebuilds exist per engine repo); replace literal path with package import. This is also the shared engine-wedge code |
| 5 | No `bin`, not on npm — no `npx rwang` story | package.json (no bin), npm E404 | **S** for bin+publish; **M** with studio/Tauri packaging |
| 6 | Author-machine literals: `C:\Users\freshair` Ollama path, G-Maiden OpenRouter referer, G-Maiden console branding | vram-mode.mjs:24, providers.mjs:607,668-669, server.mjs:154, orchestrator.mjs:96 | **S** — grep-and-fix |
| 7 | Ollama role chains name models a stranger won't have (incl. a personal HF GGUF `sillykiwi/Aroow-Rust-Coder-9B`) with no pull/bootstrap step | config.json:266,281-282,294 | **S-M** — `ollama pull` bootstrap or model-availability check in health |
| 8 | Thai-only prompts/UI strings | engine.mjs:408-444, server.mjs:154 | **M** — i18n or English default |
| 9 | Windows-first: dev.bat only launcher, N-API binding win32-only, vram tooling uses taskkill | dev.bat, knowledge.mjs:12,297, vram-mode.mjs:53 | **M-L** — Linux/mac parity gated on engine prebuilds |

**Engine-wedge overlap (the strategic answer):** the code an engine-wedge driver would need — in-process binding load, schema pin, embed shim, addNode/hybridSearch/retrieveContext/GRL wrappers, graceful file-fallback — already exists and is ~600 lines across `store/genesis-sidecar.mjs` + `store/knowledge.mjs`. Extracting that into a published `@genesisblock/knowledge-adapter` package would simultaneously (a) fix gap #4 here and (b) be the seed of the Graphiti/LangGraph driver. That is the highest-leverage shared artifact between platform and wedge in this repo.
