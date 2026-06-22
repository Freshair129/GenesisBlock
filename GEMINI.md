---
version: "1.3.0"
created_at: "2026-06-15T00:00:00+07:00,Agent: GeminiCLI"
last_update: "2026-06-22,Claude (engine-model sync)"
status: "Stable"
attributes:
  domain: "genesis-block-db"
  scope: "Project"
---

# GenesisBlock DB: Agent Instructions
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
