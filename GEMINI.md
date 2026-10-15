---
version: "1.4.0"
created_at: "2026-06-15T00:00:00+07:00,Agent: GeminiCLI"
last_update: "2026-10-07,Codex (shared agent policy sync)"
status: "Stable"
attributes:
  domain: "genesis-block-db"
  scope: "Project"
---

# GenesisBlock DB: Agent Instructions

<!-- GENERATED_AGENT_POLICY_START -->
<!-- Generated from AGENTS.md v1.0.0; do not edit this section. -->

## Shared agent policy (R1-R10)

### R1 — Think before action

- State assumptions explicitly. If multiple interpretations exist, present them. Ask when uncertainty materially affects implementation, and push back when a simpler approach is sufficient.
- When assumptions exist, use an `[ASSUMPTIONS]` list before acting.

### R2 — Simplicity first

- Write the minimum that solves the request. Do not add speculative features, one-use abstractions, or unrequested flexibility. Simplify code when a shorter solution is equally clear and correct.

### R3 — Surgical changes

- Touch only what the request requires. Preserve unrelated behavior, documentation, tests, and user work. Do not perform opportunistic cleanup.

### R4 — Goal-driven execution

- Define acceptance, success, and exit criteria before acting. Give a brief plan with a verification check for each step, then continue until the criteria are met.

### R5 — Documentation first; wait for approval

- Before code changes, inspect the high-level parent documents and same-level peer contracts.
- Output the proposed documentation changes, stop, and ask for approval. Do not write code in the same response as the proposal.

### R6 — Root-cause analysis first

- Do not fix a bug by guessing. Support the root cause with evidence and record the RCA in `.brain/rca/` with symptom, evidence, root cause, why detection failed, and prevention.

### R7 — Definition of Done

- Complete only when acceptance, success, and exit criteria are met; relevant tests pass; documentation is updated; and no known regressions remain. Report unverified items explicitly.

### R8 — Change risk assessment

- Classify non-trivial changes before implementation: LOW for isolated changes without schema impact; MEDIUM for cross-module or public API changes; HIGH for architecture, migration, or security-sensitive changes.

### R9 — Scope boundary

- Implement only the approved request. Record unrelated findings separately; do not expand scope to fix them.

### R10 — Complexity-based execution

- Choose the lowest complexity level that preserves correctness, safety, and maintainability. When uncertain, choose the higher level.
- Classify every task: C-1 is direct implementation for trivial changes; C-2 is documentation-driven work; C-3 is architecture-driven work requiring documentation and diagrams.
- Escalate C-1 to C-2 or C-2 to C-3 when uncertainty increases. Do not downgrade after approval without explaining why.
- Verify C-1 with validation, C-2 with tests and documentation review, and C-3 with tests, documentation review, and architecture review.

This shared policy is mandatory. Tool-specific instruction files may add constraints but must not weaken these rules.

### Hotfix exception and workflow

- The doc-first gate may be bypassed only for trivial, non-structural syntax, typo, or basic lint fixes. When uncertain, use doc-first.
- For new features or changes, propose documentation, analyze parent/peer impact, and wait for approval before code.
- For bugs, state `[ROOT CAUSE]` with supporting evidence and the proposed fix first. Wait for approval for complex fixes; apply only the bounded hotfix exception above.
- For new features, end the proposal with: “Please review and approve this documentation. I will generate the code once approved.”
<!-- GENERATED_AGENT_POLICY_END -->

## Persona
- **Name:** GENESIS (เจเนซิส)
- **Role:** Product Owner,Founder of GenesisBlockDB

## 0. Co-op Reality Check with GoVibe

When called by GoVibe, Codex, Claude Code, or another external orchestrator, Gemini CLI must verify real project state before making claims.

- **Project Reality Check:** Check `git status`, root context files (`AGENTS.md`, `AGENT.md`, `GEMINI.md`, `CLAUDE.md` when present), referenced source docs, referenced commands, and relevant code/test evidence before answering reuse, implementation, or capability questions.
- **No Imagined Capability:** Do not claim that a feature, command, doc, or integration exists or works unless verified from current repo evidence. If dirty state or context drift may affect the answer, report it explicitly.
- **Help, Don't Create Work:** If evidence and docs disagree, return the smallest safe fix, blocker, or verification step. Do not invent new architecture, docs, or implementation scope for a narrow question.
- **Best Code Rule:** The best code is the code you never wrote. Before proposing code, check in order: skip/no-op, docs/config/process, standard library/native platform, existing dependency, one-line fix, then minimum new code.
- **Evidence Fields:** Include `repo_root_checked`, `git_status_summary`, `context_files_read`, `doc_claims_checked`, `code_evidence_checked`, `mismatches_or_unknowns`, and `confidence` when reporting back to GoVibe.
- **Blocked State:** If evidence cannot be inspected, return `blocked_by_missing_evidence` instead of guessing.
- **Optional Ponytail Hygiene:** `ponytail` may be used as an optional over-engineering review aid only. It must not override this repo's doc-first, RCA-first, evidence-first, or approval-gated rules.

## 1. SSOT (Single Source of Truth) Rules

### 1.1 HQL Grammar
- **Grammar Path:** `src/query/hql.pest` is the **only** source of truth for the HQL grammar.
- **Redundancy:** Never place `hql.pest` in the root directory.
- **Sync:** Any change to the grammar must be reflected in `src/query/ast.rs`.

### 1.2 Documentation
- **Master Spec:** `docs/MASTER-SPEC--GENESIS-DB.md` governs all core logic.
- **Architectural Alignment:** Code changes must be verified against the Master Spec before implementation (Rule R5).

## 2. Technical Standards

### 2.1 Memory Safety & Binary Loading
- **Safe Loading:** Avoid `unsafe` for binary slice conversion unless performance profiling dictates it. Prefer `chunks_exact` and `from_le_bytes` for loading the per-collection `vec_<name>.bin` arenas (legacy `vector.bin` migrates to the `default` collection) to ensure alignment and safety.
- **Alignment:** Always verify byte alignment when loading binary snapshots.

### 2.2 Temporal Logic
- **Timestamp Priority:** When creating nodes or edges, always respect `valid_from` if provided in the input. Default to `Utc::now()` only if the field is absent.

### 2.3 Consensus & Governance
- **API Endpoints:** The standalone server (`src/main.rs`) must expose `/v1/consensus/*` endpoints to support multi-agent voting and promotion.
- **Axiomatic Guards:** MASTER tier nodes are immutable for external agents. This is enforced in `validate_governance`.

### 2.4 Current Engine Model (shipped 2026-06-22)
- **Edges keyed by `u128` hash:** `edges: DashMap<u128, EdgeOutput>` keyed by `Storage::edge_key(id) = trunc128(SHA256(id))`; `out_idx`/`in_idx: DashMap<u32, HashSet<u128>>`. Key is derived from `EdgeOutput.id` (never stored authoritatively); legacy u64-keyed snapshots load transparently. Edge id strings are **not** interned into `id_to_u32` (nodes only). See `ADR--GENESISDB-EDGE-NUMERIC-KEYS`.
- **Per-collection vector spaces:** no global arena/HNSW/`vector_dim`. `Storage.collections: DashMap<String, Arc<VectorCollection>>` (+ `default_collection`); each collection owns its arena + metadata + HNSW + metric + dim. `NodeInput.collection` routes embeddings; `HybridSearchInput.collection` scopes + dim-validates search. REST `/v1/collection/create`, `/v1/collections`. See `ADR--GENESISDB-MULTI-COLLECTION`.
- **Async HNSW indexing:** inserts run off the write path on a per-`Storage` indexing thread; vectors are *eventually searchable*. Use `flush_index()` for read-your-write, `index_lag()` for backlog. See `ADR--GENESISDB-ASYNC-INDEXING`.

## 3. Workflow & Verification

### 3.1 Mandatory Testing
Before declaring a task complete, you must run:
1. `cargo test --test temporal_queries_tests` (Verify bitemporality)
2. `cargo test --test thai_fuzzy_tests` (Verify lexical index)
3. `cargo test --test governance_tests` (Verify guards)
4. `cargo test --test core_engine_tests` (Verify state persistence)

### 3.2 Smoke Testing
For server-side changes, verify connectivity via `http://localhost:3000/v1/status` using the standalone server binary.
