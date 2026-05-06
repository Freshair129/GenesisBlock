# SPEC — Freshair Architecture v2 (Proposal)

| Field | Value |
|---|---|
| **Status** | 🟡 Drafted — pending review |
| **Author** | Claude (AI), under user direction "ออกแบบจากศูนย์" |
| **Drafted** | 2026-05-07 |
| **Scope** | MSP + GKS + satellite packages — full architectural redesign |
| **Disposition** | Discussion document — not yet implemented |
| **Successor of** | `msp_spec.md` (current spec) + `MSP_RELATIONSHIP.md` (boundary doc) |

---

## 0. TL;DR

Re-frame MSP and GKS responsibilities along **cognitive concerns** (where knowledge lives vs how an agent's mind works) instead of feature-pile partitioning. Eliminate the 24-tool overlap surface by exposing **MSP only** at the MCP layer; GKS becomes a library + admin CLI. Add **`msp_observe`** (mem0-style hot-path extraction) and split procedural memory from identity (langmem-style typology). Make **project a first-class concept** so the same MSP install works across Claude Code, Gemini CLI, and Codex without per-client config rewrites. Keep plain-text + git-versioned + auditable storage — strengths the closed-source competitors lack.

---

## 1. Background

### 1.1 What exists today

```
G:\workspace/                        ← npm workspace (Phase B 2026-05-07)
├── package.json                     workspaces: [gks, msp]
├── gks/                             @freshair129/gks 3.6.0
│   ├── 13 MCP tools (gks-mcp-server)
│   ├── CLI (`gks` bin)
│   ├── 4 storage layers (atomic / vector / graph / audit)
│   └── inbound queue, episodic, hotfix, issue tracker
└── msp/                             msp 0.4.0
    ├── 11 MCP tools (msp-mcp-server)
    ├── CLI (msp:* npm scripts)
    ├── 6 modules (validator / memory / orchestrator / identity / codegen / obsidian)
    └── 162 atoms in gks/ folder + .brain/msp/ runtime state
```

### 1.2 What's working well

- **Plain-text storage** — `git diff`, `grep`, manual editing in Obsidian all work
- **Doc-to-code workflow** — P0..P6 phase progression is unique to this stack
- **Audit trail** — AUDIT atoms record what shipped + why
- **MCP-native** — Claude Desktop integration tested in production
- **Identity layer** — first-class profile/voice/preferences with TTL
- **PROTO governance** — rules that promote draft → stable

### 1.3 What's broken

- **Tool overlap (24 tools)** — `msp_recall` thinly wraps `gks_recall`, `msp_propose` wraps `gks_propose_inbound`, `msp_validate` overlaps `gks_validate_links`. Agent sees both and gets confused.
- **No hot-path extraction** — agent must explicitly call `msp_propose` with full atom shape. Cannot say "ฟังบทสนทนาแล้วจำที่ควรจำ" — must hand-craft each atom.
- **Identity ↔ procedural conflation** — `msp_identity_set` with `kind: 'preference'` doubles as instruction storage. langmem separates these clearly.
- **Project hardcoded as `evaAI` namespace** — switching projects requires code changes, not a `cd` or env var.
- **MSP is heavy** — ~270 src files. Validator (12 rules + 9 PROTOs) and codegen (T*.task.yaml runner) are bundled. EVA cannot adopt MSP without inheriting all of it.
- **Multi-client untested** — only Claude Desktop. Gemini CLI / Codex MCP integration not verified. Config is `G:\workspace\msp` hardcoded.

### 1.4 Why the current `MSP_RELATIONSHIP.md` boundary works only partially

The split (GKS = storage, MSP = orchestrator) is the right axis. But MSP grew to wrap *every* GKS feature instead of consuming primitives selectively. Result: tools duplicate, scope blurs.

---

## 2. Vision

### 2.1 Core framing

```
GKS  =  "Where knowledge lives"        (storage substrate)
MSP  =  "How an agent's mind works"    (cognitive scaffold)
```

Decision rule for any new feature:

> **If feature-X is meaningful without an LLM in the loop → GKS.**
> **If feature-X requires LLM judgment, agent context, or runtime policy → MSP.**

### 2.2 Single principle: agents see one face

Across Claude Code, Gemini CLI, and Codex, the agent calls **one MCP server (MSP)** with **one consistent vocabulary**. GKS lives behind the curtain — exposed as a library + admin CLI, not as 13 more MCP tools an agent must learn.

### 2.3 Five layers, orthogonal concerns

```
┌─────────────────────────────────────────────────────────────────┐
│  Multi-client surface (MCP stdio)                                │
│  Claude Code · Gemini CLI · Codex · (future)                    │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│  MSP — Memory OS (single MCP endpoint per project)              │
│  6 cognitive subsystems, 6-7 exposed tools                      │
│   session · extraction · recall · identity · procedural ·        │
│   compression                                                    │
└──────────────────────────────┬──────────────────────────────────┘
                               │ library calls
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│  GKS — Genesis Knowledge System (library + admin CLI)           │
│  4 storage layers                                                │
│   atomic · vector · graph · audit                               │
│  Per-project vault, cross-vault search                          │
└─────────────────────────────────────────────────────────────────┘
                               ▲
                               │ optional, separate packages
┌──────────────────────────────┴──────────────────────────────────┐
│  Satellite tools                                                 │
│  @freshair/atom-validator    @freshair/codegen                  │
│  (PROTO + rules)             (BLUEPRINT → code)                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Layer specs

### 3.1 GKS — storage substrate

**Scope**: store atoms, query them by id / similarity / graph predicate, track audit trail of every op. Multi-tenant via projects (vaults). No LLM-driven logic.

**Public API (TypeScript)**:

```typescript
interface GKS {
  // atomic CRUD
  write(atom: Atom): Promise<WriteResult>
  read(id: AtomId, project?: string): Promise<Atom | null>
  delete(id: AtomId, project?: string): Promise<void>
  
  // search
  similar(text: string, opts?: { k?: number; threshold?: number; project?: string }): Promise<Hit[]>
  byPredicate(predicate: string, target: AtomId): Promise<Atom[]>
  walk(startId: AtomId, opts?: { depth?: number; throughSuperseded?: boolean }): Promise<Atom[]>
  
  // navigation
  backlinks(atomId: AtomId): Promise<Edge[]>
  bySymbol(filepath: string, symbol?: string, line?: number): Promise<Atom[]>
  
  // ops
  validate(opts?: { atomId?: AtomId; all?: boolean }): Promise<ValidationReport>
  audit(query: AuditQuery): Promise<AuditEntry[]>
  
  // multi-project
  projects(): Promise<Project[]>
  crossProjectSearch(text: string, k?: number): Promise<Hit[]>
}
```

**CLI** (`gks` bin):

```
gks <project> retain CONTENT [--type=insight] [--phase=N]
gks <project> recall QUERY [--top-k=N]
gks <project> walk ATOM_ID [--depth=N]
gks <project> backlinks ATOM_ID
gks <project> validate [--all]
gks <project> audit --since=24h
gks projects list
gks projects new <name>
gks cross-project recall QUERY
```

**MCP server** — *optional*, for low-level admin. Default config doesn't enable it. Most users only need MSP MCP.

**Storage layout** (per project):

```
~/.freshair/projects/<name>/
├── atoms/              gks/<type>/*.md (markdown + frontmatter)
├── index/              atomic_index.jsonl (derived)
├── vector/             vector store (configurable backend)
├── audit/              immutable log (jsonl, append-only)
└── config.yaml         project metadata + atom contract
```

### 3.2 MSP — Memory OS

**Scope**: capture conversations, extract memories, manage agent identity + procedural knowledge, serve runtime context. LLM-driven where it matters.

**6 cognitive subsystems**:

| Subsystem | Owns | Calls GKS for |
|---|---|---|
| **Session** | session.jsonl turn writer + lock | — |
| **Extraction** | hot-path + background fact extraction (LLM-driven) | `gks.write`, `gks.similar` |
| **Recall** | retrieval orchestrator (RRF fusion across 4 sources) | `gks.similar`, `gks.walk`, `gks.backlinks` |
| **Identity** | profile + voice (slow-changing) | — (own json store) |
| **Procedural** | INSTR--<topic> atoms — agent's evolving system prompt | `gks.write`, `gks.read` |
| **Compression** | 3-tier fit-to-budget (keep/trim/resummarise/truncate) | — (pure transform) |

**Exposed MCP tools (6-7)**:

```
msp_session_open(episodic_id?, project?)        start session, return session_id
msp_session_append(turn)                         log one turn
msp_session_close(session_id)                    close + queue background consolidation

msp_observe(messages, hints?)         ★ NEW    hot-path extraction
                                                 → calls Extraction → reconciles → writes via GKS
                                                 → returns { added, updated, skipped, audit }

msp_recall(query, opts)                          orchestrated retrieval
                                                 → fans out to 4 sources, RRF fuses
                                                 → optional auto-compress to budget

msp_identity_get(project?)                       full identity (profile + voice + procedural prompt)
msp_identity_set(kind, value, ...)               kind: profile | voice | preference

msp_procedural_set(topic, instruction, ttl?)     ★ NEW    write INSTR atom
msp_procedural_compose(query?)                   ★ NEW    return current system-prompt overlay
                                                 (concatenated from active INSTRs, ranked)
```

**What was removed compared to current MSP MCP**:

| Removed tool | Why | Replacement |
|---|---|---|
| `msp_propose` | Wraps `gks_propose_inbound`; agents shouldn't hand-craft atoms | `msp_observe` (LLM extracts) |
| `msp_validate` | Wraps `gks_validate_links` + adds rules; rules now in atom-validator package | CLI: `atom-validate` (called by hooks/CI, not agents) |
| `msp_episode_append` | Direct write — should go through extraction | `msp_observe` or session pipeline |
| `msp_backlinks_rebuild` | Admin op — not agent territory | CLI: `gks backlinks rebuild` |

**CLI** (`msp` bin):

```
msp project switch <name>
msp project current
msp session list
msp recall "query"
msp identity show
msp observe < transcript.txt          # batch-extract from a file
```

### 3.3 atom-validator — satellite package

**Scope**: project-specific atomic governance (rules + PROTOs).

```
@freshair/atom-validator
├── src/rules/                  12 rules (forbidden-fields, future-date, ...)
├── src/proto/                  9 governance predicates (PHASE-GATES, ALGO-PARAM-COUPLING, ...)
└── bin/                        atom-validate CLI
```

**Why separate**: Different projects might want different rules. EVA's atomic conventions differ from MSP's. Pinning rules to MSP forces every consumer to inherit them.

**Usage**: hooks (pre-commit), CI, manual. Not exposed via MCP — too noisy for agents.

### 3.4 codegen — satellite package

**Scope**: BLUEPRINT → code via SLM (T*.task.yaml runner + acceptance vitest sandbox).

```
@freshair/codegen
├── src/                        runner, prompt-builder, post-process, slm clients
└── bin/                        codegen CLI
```

**Why separate**: Most projects don't do code-gen. Even if they do, codegen is its own concern (LLM orchestration for code synthesis, not memory).

**Usage**: explicit invocation via CLI or wrapper MCP tool in MSP that delegates.

---

## 4. Seven design decisions, expanded

### 4.1 Eliminate tool overlap — MSP-only at MCP layer

**Now**: 11 MSP tools + 13 GKS tools, with ~5 overlapping functionality. Agent picks "wrong" one ~30% of the time.

**Proposed**: Agent sees only MSP tools (6-7). MSP code calls GKS as a library. GKS MCP exists but is opt-in for low-level admin.

**Trade-off**: Agent loses direct access to `gks_lookup_by_symbol`, `gks_hotfix_open`, `gks_new_feature`. If those are needed, wrap in MSP (`msp_lookup_code_origin`, `msp_hotfix_*`, `msp_scaffold_feature`) — but only after observing genuine need.

### 4.2 Hot-path extraction (`msp_observe`) — mem0 pipeline pattern

**Pipeline inside `msp_observe`**:

```
input: { messages: Turn[], hints?: { focus: 'fact'|'preference'|'goal' } }

1. Filter — drop boilerplate / greetings / tool calls
2. Extract LLM call — "What's worth remembering here?" → list of facts (typed)
3. For each fact:
   3a. gks.similar(fact.text, { k: 3, threshold: 0.85 }) → existing matches
   3b. Reconcile (rule-based first, LLM only if ambiguous):
       - No match → ADD (new atom)
       - Exact match → NOOP
       - Near match, contradicts → SUPERSEDE (write new + mark old superseded_by)
       - Near match, refines → UPDATE (merge into existing)
4. Batch write via gks.write
5. Append audit entry per op
6. Return { added: AtomId[], updated: AtomId[], superseded: AtomId[], skipped: number, audit_id }
```

**Key**: agent doesn't care about atom IDs / phases / types. Just dumps conversation, MSP figures it out.

**Why this matters**: It's the difference between "AI with good memory" (mem0 promise) and "structured note-taker" (current MSP). The current MSP is great if the user is disciplined. `msp_observe` makes it work for normal use.

### 4.3 Separate procedural memory from identity (langmem typology)

**Now**: `msp_identity_set { kind: 'preference', key: 'use_ollama', value: true, expires_at: ... }`

**Proposed**:

| Type | Storage | Lifecycle | Examples |
|---|---|---|---|
| **Identity** | `identity.json` (jsonl) | slow-changing, single source | name, role, tier, guardrails, voice |
| **Procedural** | `INSTR--<topic>` atoms in GKS | evolves over time, versioned | "When asked about X, prefer Y", "Always cite source" |
| **Semantic** | atoms in GKS (CONCEPT/ADR/etc.) | append-mostly | facts, decisions, knowledge |
| **Episodic** | episodic_memory.json + session.jsonl | append-only | conversations, events |

**Why split matters**: Identity is "who I am" (rare changes). Procedural is "how I act" (continuous learning). Bundling them in `preferences` means a TTL on identity isn't meaningful, and version history of an instruction isn't queryable.

**`msp_procedural_compose`** returns the current overlay to inject into system prompt:

```
msp_procedural_compose({ query: "user is asking about clinic data" })
→ "Active instructions for this context:
   1. INSTR--CLINIC-PRIVACY: Never echo patient identifiers.
   2. INSTR--PROFESSIONAL-TONE: Use formal Thai for clinical contexts.
   ..."
```

### 4.4 Project = first-class

**Now**: hardcoded `namespace=evaAI`. Switching projects = code change.

**Proposed**:

```yaml
# ~/.freshair/projects.yaml
projects:
  default:
    path: ~/.freshair/projects/default
    embedder: nomic-embed-text-v1.5
  eva:
    path: G:/evaAI/projects/eva
    embedder: nomic-embed-text-v1.5
  msp:
    path: G:/workspace/msp
    embedder: nomic-embed-text-v1.5
  clinic:
    path: G:/clinic
    embedder: nomic-embed-text-v1.5
```

**Project resolution** (priority order):

1. `--project=<name>` CLI flag
2. `MSP_PROJECT` env var
3. `.mspconfig` file in cwd or ancestor (TOML/YAML, single line: `project: eva`)
4. fallback `default`

**Cross-project search**: built-in via `msp_recall { cross_project: true }`.

**Migration**: existing `evaAI` namespace becomes the `default` project; old paths still work via aliases.

### 4.5 Multi-client = global install + per-client config pointing to one bin

**Now**: `claude_desktop_config.json` contains `G:\\workspace\\msp\\dist\\mcp\\bin.js`. Move the repo, config breaks.

**Proposed**:

```bash
npm install -g @freshair/msp @freshair/gks
```

Each client config references the *global* bin name:

```jsonc
// Claude Code
{
  "mcpServers": {
    "msp": { "command": "msp-mcp-server" }
  }
}

// Gemini CLI (~/.gemini/config.json)
{
  "mcpServers": {
    "msp": { "command": "msp-mcp-server" }
  }
}

// Codex (~/.codex/mcp.toml)
[servers.msp]
command = "msp-mcp-server"
```

Project resolution from cwd / env / .mspconfig — same logic in all clients.

**Verification matrix** (must pass before declaring v2 stable):

- [ ] `msp_recall` works in Claude Code (Windows, macOS, Linux)
- [ ] `msp_recall` works in Gemini CLI
- [ ] `msp_recall` works in Codex
- [ ] `msp_observe` round-trips in all 3
- [ ] Project switch via env var works in all 3

### 4.6 Validators + Codegen as satellites

**Now**: Bundled in MSP (~30% of MSP source by LoC).

**Proposed**: separate packages, MSP has zero direct dep on either.

**Wiring**:

```
# MSP runtime (atomic write goes through validator)
msp config:
  validator: '@freshair/atom-validator'   # optional
  codegen:   '@freshair/codegen'          # optional
```

If `validator` isn't installed, MSP writes atoms without rule checks (only basic schema). User sets it up via:

```bash
npm install -g @freshair/atom-validator
```

**Benefit**: EVA can adopt MSP without inheriting MSP's project-specific rules. EVA writes its own validator package (`@freshair/eva-validator`).

### 4.7 AuditLog as primitive + AUDIT atoms as narrative

**Now**: `AUDIT--<topic>.md` atoms (phase 6) — narrative records of "what shipped". GKS has internal AuditLog class but isn't exposed.

**Proposed**: keep both, give them clear roles.

| Layer | Granularity | Format | Use |
|---|---|---|---|
| **AuditLog** (primitive) | per-op (write/update/delete) | jsonl, append-only | "what changed in last 24h?", debugging, compliance |
| **AUDIT atoms** (narrative) | per-feature / per-fix | markdown | "post-mortem of X feature", traceable cause-and-effect |

**Exposed via MSP**:

```
msp_audit_query({ since, atom_id?, actor?, project? })
msp_audit_summary({ period, project? })   # "this week ใน project นี้ทำอะไรไปบ้าง"
```

AUDIT atoms continue to be created via `msp_observe` (when significant) or manually for milestones.

---

## 5. Folder structure (target)

```
G:\workspace/                       (or wherever — global install means path-agnostic)
├── package.json                    npm workspaces root
├── packages/
│   ├── gks/                        @freshair/gks
│   │   ├── src/
│   │   │   ├── atomic/             read/write atoms
│   │   │   ├── vector/             embedding + similarity
│   │   │   ├── graph/              crosslinks + walk
│   │   │   ├── inbound/            queue (low-level)
│   │   │   ├── audit/              immutable log
│   │   │   └── mcp-server/         (optional standalone)
│   │   └── bin/                    gks CLI
│   │
│   ├── msp/                        @freshair/msp           ← THIN core
│   │   ├── src/
│   │   │   ├── session/            JSONL turn writer
│   │   │   ├── episodic/           consolidation (Tier 1+2)
│   │   │   ├── identity/           profile + voice
│   │   │   ├── procedural/         INSTR atom mgmt + composer
│   │   │   ├── extraction/         ★ NEW: hot-path pipeline
│   │   │   ├── recall/             RRF orchestrator
│   │   │   ├── compression/        3-tier
│   │   │   └── mcp-server/         6-7 tools (no GKS overlap)
│   │   └── bin/                    msp CLI
│   │
│   ├── atom-validator/             @freshair/atom-validator
│   │   ├── src/rules/              12 rules
│   │   ├── src/proto/              9 PROTOs
│   │   └── bin/                    atom-validate CLI
│   │
│   └── codegen/                    @freshair/codegen
│       ├── src/                    T*.task.yaml runner
│       └── bin/                    codegen CLI

~/.freshair/                        per-user state
├── projects.yaml                   projects registry
└── projects/
    ├── default/
    ├── eva/
    ├── msp/
    └── clinic/
        ├── atoms/                  gks/<type>/*.md
        ├── memory/                 .brain/<ns>/session, episodic, identity
        ├── audit/                  AuditLog jsonl
        └── config.yaml
```

---

## 6. Migration path — no rewrite, incremental

Six sprints, each independently shippable. Each has clear AUDIT atom on completion.

### Sprint 1 — Tool surface cleanup (1-2 days)

**Goal**: MSP MCP exposes only non-overlapping tools.

- Mark `msp_propose`, `msp_validate`, `msp_episode_append`, `msp_backlinks_rebuild` as deprecated in tool descriptions
- Update CLAUDE.md / docs to point at GKS MCP for direct primitive access
- Ship MSP minor version bump

### Sprint 2 — `msp_observe` (1 week)

**Goal**: hot-path extraction working end-to-end.

- New module `src/extraction/`
- Pipeline: filter → extract LLM → reconcile → batch write
- Mock provider works for tests; ollama/anthropic for real use
- Replace `msp_remember` (background) with `msp_episode_consolidate` for clarity

### Sprint 3 — Package split (3-5 days)

**Goal**: validator + codegen as separate packages.

- Move `src/validator/` → `packages/atom-validator/`
- Move `src/codegen/` → `packages/codegen/`
- MSP package.json deps reduce by ~30%

### Sprint 4 — Project as first-class (3-5 days)

**Goal**: project switching via env / .mspconfig / CLI.

- New `~/.freshair/projects.yaml`
- `msp project` subcommands
- Migration from existing `namespace=evaAI` to `project=default`
- All MCP tools accept optional `project` param

### Sprint 5 — Procedural memory (3-5 days)

**Goal**: INSTR atom type + composition into runtime prompt.

- New atom type `INSTR--*` registered in atom-validator
- `msp_procedural_set/get/compose` MCP tools
- Migrate existing "preferences with TTL > short" → procedural

### Sprint 6 — Multi-client verification (1 week)

**Goal**: confirmed working on Claude Code + Gemini CLI + Codex.

- Global install workflow documented
- Per-client config snippets in `examples/clients/`
- Smoke tests for each (CI-friendly)
- Verification matrix in §4.5 all green

---

## 7. Open questions

| # | Question | Owner |
|---|---|---|
| Q1 | Does the user already use Gemini CLI / Codex regularly, or is "future-proof" the goal? | user |
| Q2 | Should `msp_observe` extraction LLM be configurable per-project, or per-call? | design |
| Q3 | When does an INSTR supersede a previous one — automatic (MSP detects contradiction) or explicit (user calls `_supersede`)? | design |
| Q4 | Should AUDIT atoms be auto-generated from AuditLog summaries (e.g. weekly), or stay manual? | design |
| Q5 | Does EVA migrating to use this MSP cleanly handle EVA's own atom types (algorithms, parametrics, incidents, ...)? | EVA-side investigation |
| Q6 | Cross-project recall — does it require a shared embedder model across all projects? | architecture |
| Q7 | What's the back-compat story for existing 162 atoms in `gks/` folder of current MSP? | migration |

---

## 8. Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| EVA's existing memory subsystem (`eva-cli/src/memory/gks.ts`) diverges further during this rewrite | Medium | Pause EVA refactor until MSP v2 ships; then EVA migrates to import @freshair/gks |
| `msp_observe` extraction quality is poor on Thai-mixed content | Medium | Test against current corpus; tune extraction prompt; fall back to manual when uncertain |
| Multi-client config drift (Claude Code config diverges from Gemini CLI) | Low | Generate configs from single template; lint as part of CI |
| AUDIT trail growth fills disk | Low | Compress old logs; document retention policy in atom-validator rules |
| Loss of existing identity-store TTL semantics during procedural split | Low | Migration script (Sprint 5) preserves expirable preferences |

---

## 9. Comparison vs status quo + alternatives

### vs current MSP/GKS (status quo)

| Dimension | Status quo | v2 |
|---|---|---|
| Tools agent sees | 11 (overlap with 13 GKS) | 6-7 (clean) |
| Hot-path extraction | ❌ | ✅ `msp_observe` |
| Procedural memory | conflated with prefs | ✅ separate type |
| Project switching | code change | env var / .mspconfig |
| Validators bundled | yes (heavy MSP) | satellite (thin MSP) |
| Multi-client tested | Claude Desktop only | Claude Code + Gemini + Codex |
| Migration cost | N/A | 6 sprints, each shippable |

### vs merge MSP+GKS

| | Merge | v2 |
|---|---|---|
| Solves tool overlap | ✅ | ✅ |
| Solves "1 install" | ✅ (already solved by workspace) | ✅ |
| EVA can adopt | ❌ (must take whole monolith) | ✅ (imports gks only, satellites optional) |
| Aligns with FRAMEWORK_MASTER_SPEC vision | ❌ | ✅ |
| Implementation cost | high (rewrite imports, reconcile types) | medium (incremental) |

### vs use mem0/langmem instead

| | mem0 | langmem | v2 |
|---|---|---|---|
| Plain-text auditable | ❌ | ❌ | ✅ |
| Multi-client (3 CLI agents) | partial (REST API) | LangGraph-only | ✅ MCP-native |
| Identity first-class | ❌ | ⚠️ partial | ✅ |
| Doc-to-code workflow | ❌ | ❌ | ✅ (via codegen satellite) |
| PROTO governance | ❌ | ❌ | ✅ (via atom-validator satellite) |
| Hosted option | ✅ | ❌ | ❌ (self-host only) |
| Auto extraction pipeline | ✅ | ⚠️ partial | ✅ `msp_observe` |

---

## 10. Decision log (during drafting)

| Decision | Rejected alternative | Reason |
|---|---|---|
| MSP-only at MCP layer | Both MSP+GKS exposed | Tool overlap, agent confusion |
| 4 GKS layers (atomic / vector / graph / audit) | Add `episodic` as 5th GKS layer | Episodic is conversation-shaped, not knowledge-shaped — belongs in MSP |
| Procedural separated from identity | Keep within `preferences` | langmem typology proves clearer model |
| Project as first-class | Keep namespace as opaque field | Multi-project users (clinic, eva, msp itself) need 1st-class switching |
| Validator/codegen as satellites | Keep in MSP core | Different projects → different rules; EVA shouldn't inherit MSP-specific rules |
| Global install + per-client config | Per-project install | Multi-client (Claude/Gemini/Codex) needs single source of bin |
| 6 sprints incremental | Big-bang rewrite | Big rewrites fail; each sprint has its own AUDIT atom |

---

## 11. References

### Internal
- [`msp_spec.md`](./msp_spec.md) — current full spec
- [`gks/frame/FRAME--MSP-ARCHITECTURE-V2.md`](./gks/frame/FRAME--MSP-ARCHITECTURE-V2.md) — current architecture frame
- [`gks/audit/AUDIT--GKS-3-6-0-PUBLISHED.md`](./gks/audit/AUDIT--GKS-3-6-0-PUBLISHED.md) — recent migration audit
- [`G:\evaAI\FRAMEWORK_MASTER_SPEC.md`](file:///G:/evaAI/FRAMEWORK_MASTER_SPEC.md) — EVA project's master framework
- [`upstream/gks-proposals/README.md`](./upstream/gks-proposals/README.md) — GKS upstream workflow

### External
- mem0 architecture: https://docs.mem0.ai
- langmem typology: https://langchain-ai.github.io/langmem/
- MCP spec: https://spec.modelcontextprotocol.io

---

## 12. Next step

This document is a **proposal**. Suggested response paths:

1. **Accept-as-is** → write `AUDIT--ARCHITECTURE-V2-ACCEPTED.md` and start Sprint 1
2. **Accept-with-amendments** → comment on PR with specific section edits
3. **Reject + counter** → write a different SPEC that supersedes this one
4. **Park** → leave as-is, revisit when next pain point surfaces (e.g. when Gemini CLI integration becomes urgent)

No work begins until decision is made. The current architecture (post-Phase-B workspace) is stable enough to live in for weeks/months while this is reviewed.

— end of SPEC v2 draft —
