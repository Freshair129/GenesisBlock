# Knowledge Types — canonical reference

> The atomic-knowledge prefix taxonomy GKS recognises. Every `.md` file
> under `gks/` should have a frontmatter `id: TYPE--SLUG` whose `TYPE`
> appears in this document. ADR-012 records why this list exists.
> **Taxonomy v2.3** (2026-05-13) redefines `GENESIS--` as Block Manifest,
> introduces `FRAMEWORK--` for the prior `FRAME--` meaning, renames
> `GUARDRAIL--` to `GUARD--`, and adds engine-layer prefixes (`STACK--`,
> `SPEC--`, `SAFETY--`, `COGNITIVE--`). Canonical migration record:
> `ADR--TAXONOMY-V2-3-MIGRATION` in the MSP atom tree.

> **Genesis Block disambiguation**: "Genesis Block" appears with two
> meanings in this repo. (1) **Genesis Graph Backend** — the embedded
> graph DB at `packages/gks/src/memory/graph/genesis-graph.ts` (Cypher
> v0, JSONL log); see `CONCEPT--GENESIS-GRAPH-BACKEND`. (2) **Knowledge
> Block** — a composite knowledge unit (GENESIS-- manifest + Cognitive +
> Algo + Guard atoms); see `SPEC--GENESIS-BLOCK-MANIFEST`. These are
> orthogonal: a Genesis Block can be stored in a Genesis Graph Backend
> but is not the same thing.

This is the **reference** — when you ask "where does this concept go?"
the answer is here. Templates for each prefix live in
[`examples/atom-templates/`](../examples/atom-templates/).

## Meta Learning Loop (MLL) & 4D Completeness

Taxonomy v2.4 introduces the **Meta Learning Loop (MLL)** as the governing framework for knowledge evolution.

### 4D Completeness Matrix

For any knowledge unit to be considered **Master** tier or **Stable**, it must ideally satisfy the **4D Completeness** rule by having at least one atom of each type in its cluster:

1. **Algo** (`ALGO--`): Deterministic steps.
2. **Concept** (`CONCEPT--`): Semantic definition.
3. **Frame** (`FRAMEWORK--`): Cognitive context.
4. **Proto** (`PROTOCOL--` / `PROTO--`): Invariant / Interaction contract.

### MLL Metadata

Atoms generated or refined by the MLL may include the following metadata fields:

- `mll_stability_score`: 0.0 - 1.0 (consensus across multiple LLMs).
- `mll_tension_detected`: Boolean (if drift was detected via 12-Stage pipeline).
- `mll_source_episodes`: List of session IDs that contributed to this atom.

## Quick lookup

| Prefix | Cluster | Role |
|---|---|---|
| `IDEA--` | Implementation | Raw prompt / spark |
| `CONCEPT--` | Implementation | PRD / roadmap (human-readable) |
| `ADR--` | Implementation | Architecture decision record |
| `MOD--` | Implementation | Module manifest |
| `FEAT--` | Implementation | Feature spec (user-facing behaviour) |
| `ALGO--` | Implementation | Algorithm definition |
| `FLOW--` | Implementation | Data / UI flow |
| `ENTITY--` | Implementation | Data schema |
| `API--` | Implementation | OpenAPI master hub |
| `ENDPOINT--` | Implementation | One API path / method |
| `ENTRYPOINT--` | Implementation | Auth / middleware / access logic |
| `PARAMS--` | Implementation | Constants / business config |
| `GENESIS--` | Implementation | **(v2.3+)** Block Manifest — runtime entry-point of a Genesis Block |
| `FRAMEWORK--` | Implementation | **(v2.3+)** Governance / architectural framework (was `FRAME--` pre-v2.3) |
| `STACK--` | Implementation | Technology stack — language/runtime/library inventory |
| `SPEC--` | Implementation | Specification — JSON Schema, API data shape, wire format. First example: `SPEC--GENESIS-BLOCK-MANIFEST` (frontmatter contract for `GENESIS--` Block Manifests). |
| `COGNITIVE--` | Implementation | Mental model / interpretive lens (e.g. Erikson stages) |
| `SAFETY--` | Governance | Ethical safety — AI alignment + behavioural guardrails |
| `MASTER--` | Implementation | Root-level policy / genesis rule (e.g. contradiction policy, write boundaries) |
| `PROTO--` | Implementation | Machine-enforced invariant — short rule the validator checks at write time |
| `BLUEPRINT--` | Implementation | Implementation plan (YAML or Markdown + frontmatter) |
| `AUDIT--` | Implementation | Test results / quality report |
| `HOTFIX--` | Ops | Hotfix escape-hatch atom (48h backfill window — ADR-014) |

> **Microtasks (`T*.task.yaml`) are not atoms.** Live task state belongs to
> the orchestrator (ADR-015) — see `docs/MSP_RELATIONSHIP.md` for the contract
> and `gks new-feature --task-tracker=…` for the integration points.
| `SKILL--` | Governance | Agent capability |
| `PROTOCOL--` | Governance | Interaction contract |
| `GUARD--` | Governance | Enforced behavioural policy (renamed from `GUARDRAIL--` in v2.3) |
| `POLICY--` | Governance | Operational policy |
| `PERSONA--` | Governance | Agent identity / role |
| `REQ--` | Requirements | Umbrella requirement |
| `FR--` | Requirements | Functional requirement |
| `NFR--` | Requirements | Non-functional requirement |
| `CONSTRAINT--` | Requirements | Hard external constraint |
| `INC--` | Ops | Incident post-mortem |
| `ISSUE--` | Ops | Live issue tracker (self-hosted) |
| `RISK--` | Ops | Risk + mitigation |
| `RUNBOOK--` | Ops | Operational response guide |
| `SLO--` | Ops | Service-level objective |
| `INSIGHT--` / `FACT--` / `RULE--` | Memory | Auto-extracted by Consolidator |

## Cluster 1 — Implementation Flow

Phase-aligned with the build pipeline (P0 → P6) — the **Block Assembly**
half of the Genesis Block Cycle (see `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md`).
When in doubt, this is where most contributions go.

### `IDEA--` · raw spark

- **Use for:** the original prompt / one-line idea before any analysis.
- **Don't use for:** anything that's been triaged into a concept — promote to `CONCEPT--`.
- **Phase:** P0.
- **Lifecycle:** typically short-lived; promoted into `CONCEPT--` within days.

### `CONCEPT--` · human-readable requirement / vision

- **Use for:** PRDs, journeys, roadmaps, ROI analyses written for human review.
- **Don't use for:** technical specifications — those are ADRs / FEATs / ALGOs / blueprints.
- **Phase:** P1.
- **Examples:** `CONCEPT--PRD.md`, `CONCEPT--ROADMAP.md`, `CONCEPT--JOURNEY-CHECKOUT.md`.

### `ADR--` · architecture decision record

- **Use for:** every load-bearing technical or organisational decision (and its alternatives).
- **Don't use for:** *what* the system does (FEAT) or *how* it computes things (ALGO).
- **Phase:** P2.
- **Lifecycle:** set-once, mostly stable; superseded by another ADR if revisited.
- **Status values:** `proposed` / `accepted` / `superseded` / `rejected`.

### `MOD--` · module manifest

- **Use for:** declaring a module's scope, public API, and ownership boundary.
- **Don't use for:** the code itself (P5 `src/`).
- **Phase:** P2.

### `FEAT--` · feature spec

- **Use for:** user-facing system behaviour ("when user clicks X, system Y").
- **Don't use for:** agent capabilities — those are `SKILL--`.
- **Phase:** P2.

### `ALGO--` · algorithm

- **Use for:** computational steps, scoring formulas, ranking logic.
- **Don't use for:** API contracts (use `API--`) or features (use `FEAT--`).
- **Phase:** P2.

### `FLOW--` · data / UI flow

- **Use for:** data movement diagrams, UI navigation flows, sequence-style flows.
- **Don't use for:** call graphs (let GitNexus / similar handle that — see ADR-009).
- **Phase:** P2.
- **Tip:** also valid as `.canvas` files for Obsidian Canvas diagrams.

### `FR--` · functional requirement

- **Use for:** specific functional needs that code symbols must implement.
- **Don't use for:** high-level concepts (use `CONCEPT--`).
- **Phase:** P1.

### `NFR--` · non-functional requirement

- **Use for:** quality attributes like performance, security, or reliability.
- **Don't use for:** behavioral rules (use `GUARD--`).
- **Phase:** P1.

### `ENTITY--` · data schema

- **Use for:** data model / DB schema definitions.
- **Don't use for:** API request/response shapes (use `ENDPOINT--`).
- **Phase:** P2.

### `API--` · OpenAPI master hub

- **Use for:** the single canonical OpenAPI document referencing all endpoints.
- **Don't use for:** individual endpoints — those split into `ENDPOINT--`.
- **Phase:** P2.

### `ENDPOINT--` · one API path / method

- **Use for:** a single HTTP method × path's contract (request, response, errors).
- **Don't use for:** the cross-cutting auth/middleware (use `ENTRYPOINT--`).
- **Phase:** P2.

### `ENTRYPOINT--` · auth / middleware / access logic

- **Use for:** how requests enter the system (auth gates, rate limits, tenant resolution).
- **Don't use for:** business logic that runs after the entrypoint (use `FEAT--` / `ALGO--`).
- **Phase:** P2.

### `HOOK--` · event-driven entrypoint

- **Use for:** webhooks, git hooks (pre-commit/pre-push), and system event listeners.
- **Don't use for:** standard API endpoints (use `ENDPOINT--`).
- **Phase:** P2.

### `PARAMS--` · constants / business config

- **Use for:** business-meaningful numbers, threshold lists, configuration tables.
- **Don't use for:** infra constants (Postgres pool size etc.) — those live in `ops/` configs.
- **Phase:** P2.

### GENESIS-- · Block Manifest (v2.3+)

- **Use for:** the runtime entry-point of a **Genesis Block** — a manifest atom that aggregates `COGNITIVE--`, `ALGO--`, `GUARD--` (and optionally `RUNBOOK--`, `PROTOCOL--`, `STACK--`, `SAFETY--`) atoms into a composite knowledge engine.
- **Don't use for:** governance/architecture frameworks — those moved to `FRAMEWORK--` in v2.3.
- **Frontmatter contract:** `SPEC--GENESIS-BLOCK-MANIFEST` — declares the `members.core` / `members.optional` / `daci:` / `manifest_version:` shape.
- **Phase:** P0 (Block Manifests are foundational).
- **Status cascade:** `status(block) = min(status(member))` — see SPEC §4.2.
- **Examples (proposed):** `GENESIS--IDENTITY-ENGINE` aggregates the identity-resolution Genesis Block.

### `FRAMEWORK--` · governance / architectural framework (v2.3+)

- **Use for:** architectural patterns, governance frameworks, higher-level invariant methodologies (Knowledge 3-Tier model, JTBD, Design Thinking, phase governance), and code standards ("all DB calls go through repositories", "components ≤ 500 LOC", lint policy).
- **Don't use for:** runtime behavioural constraints — those are `GUARD--`. For Block Manifest entries, use `GENESIS--`.
- **Phase:** P0/P2.
- **Examples:** `FRAMEWORK--MSP-ARCHITECTURE-V2`, `FRAMEWORK--PHASE-GOVERNANCE`, `FRAMEWORK--SCALING-LEVELS`, `FRAMEWORK--AUTHORITY-MATRIX`.
- **Renamed from:** `FRAME--` (pre-v2.3). The taxonomy migration script handles existing references.

### `STACK--` · technology stack inventory (v2.3+)

- **Use for:** the language/runtime/library/tool inventory used by a subsystem or Genesis Block — e.g. "React 18 + Rust + napi-rs + pgvector".
- **Don't use for:** module boundaries (use `MOD--`) or build config (use `PARAMS--`).
- **Phase:** P2.
- **Distinguishing question:** *is this a list of tools/runtimes a thing uses?* → if yes, STACK.

### `SPEC--` · technical specification (v2.3+)

- **Use for:** JSON Schema, API data shape, wire format, frontmatter contract.
- **Don't use for:** decisions (use `ADR--`) or behavioural rules (use `PROTO--`).
- **Phase:** P2.
- **Examples:** `SPEC--GENESIS-BLOCK-MANIFEST` (frontmatter contract for GENESIS-- atoms).

### `COGNITIVE--` · mental model / interpretive lens (v2.3+)

- **Use for:** psychological / cognitive-science models the system reasons with — Erikson stages, Ego Death, Qualia, retrieval-augmented attention, etc.
- **Don't use for:** algorithmic procedures (use `ALGO--`) or product framings (use `CONCEPT--`).
- **Phase:** P1/P2.
- **Distinguishing question:** *is this a lens through which the agent interprets state?* → if yes, COGNITIVE.

### `SAFETY--` · ethical safety / AI alignment (v2.3+)

- **Use for:** alignment rules, ethical guardrails, PII handling, behaviour-shaping rules ("do not assist with X without consent").
- **Don't use for:** structural data invariants (use `GUARD--`) or operational policy (use `POLICY--`).
- **Phase:** P0/P2 (safety rules are typically foundational).
- **Distinguishing question:** *would violating this be unethical or unsafe, not just incorrect?* → if yes, SAFETY.

> `MOD--` was added to the v2.3 prefix set but its long-form definition lives in Cluster 1 above (next to `FEAT--` / `ALGO--`) since it's been in the taxonomy since pre-v2.3.

### `MASTER--` · root-level policy / genesis rule

- **Use for:** the small set of root-level invariants that the rest of the atom graph defers to — contradiction policy, write boundaries, atom-body schema, supersession rules.
- **Don't use for:** code-level standards (use `GENESIS--`) or write-time validator rules (use `PROTO--`).
- **Distinguishing question:** *is this the rule that other rules cite?* → if yes, MASTER.
- **Phase:** P0/P2 (depends on whether it's a genesis axiom or a derived policy).
- **Examples:** `MASTER--ATOM-CONTRADICTION-POLICY`, `MASTER--ATOM-BODY-SCHEMA`.
- **Lifecycle:** very stable; supersession requires high-bar ADR.

### `PROTO--` · machine-enforced invariant

- **Use for:** a single short rule the **validator** checks at write/commit time, with `severity: error` and a `linked_symbols` pointer to the enforcement code.
- **Don't use for:** runtime agent constraints (use `GUARD--`), interaction contracts (use `PROTOCOL--`), or operational responses (use `RUNBOOK--`).
- **Distinguishing question:** *is the rule mechanically checkable by code at write-time?* → if yes, PROTO.
- **Phase:** P2.
- **Examples:** `PROTO--ADR-MONOTONIC` (ADR-NNN = max+1), `PROTO--PHASE-GATES`, `PROTO--SCALING-LEVEL-GATE`, `PROTO--VALID-UNTIL`.
- **Relationship to other types:**
  - `PROTO--` enforces a rule at **write time** (validator); `GUARD--` enforces at **runtime** (agent action / data invariant); `POLICY--` is **operational** (access/retention/rate-limit).
  - `PROTO--` often *implements* a `MASTER--` policy in machine-checkable form.

### `BLUEPRINT--` · implementation plan

- **Use for:** the YAML plan that microtask codegen consumes.
- **Don't use for:** prose specs — those are `FEAT--`.
- **Phase:** P3.
- **Required fields:** `metadata`, `architectural_pattern`, `data_logic`, `geography`, `api_contracts`, `verification_plan`.

### Microtasks (`T*.task.yaml`) — **not atoms**

- **Why:** task state churns hourly (assigned / in-progress / blocked /
  done), accumulates comments, and has zero retrieval value once shipped.
  Atoms are durable knowledge with settling time; tasks are
  execution state that belongs at the orchestrator layer (ADR-015).
- **Where they live:** `.brain/<ns>/tasks/<slug>/T<n>_<name>.task.yaml`
  for self-hosted projects, `msp/projects/<id>/tasks/` for MSP-layered
  projects, or an external tracker (Linear/Jira/Asana) keyed off
  `BLUEPRINT.geography`.
- **Integration with GKS:** `BLUEPRINT--` declares the *shape* of the
  work (file paths, acceptance criteria, architectural pattern);
  `AUDIT--` records the *outcome* once the task closes. Both are
  durable. Live status in between is the orchestrator's job — see
  `docs/MSP_RELATIONSHIP.md` for the contract.

### `AUDIT--` · test results / quality report

- **Use for:** sign-off documents recording verification outcomes.
- **Don't use for:** the *plan* (that's `BLUEPRINT.verification_plan`); only the *result*.
- **Phase:** P6.

## Cluster 2 — Agent Governance

These prefixes were missing from the original taxonomy; they exist
because every agentic project hits them within weeks. See ADR-012 for
rationale.

### `LLM--` · large reasoning engine

- **Use for:** high-reasoning model configurations, system prompts, and architectural planning logic.
- **Don't use for:** small execution tasks (use `SLM--`).
- **Phase:** P2.

### `SLM--` · small execution engine

- **Use for:** specialized models for codegen, summarization, or microtasks.
- **Don't use for:** heavy architectural reasoning (use `LLM--`).
- **Phase:** P5.

### `MCP--` · Model Context Protocol tool

- **Use for:** technical definition of a tool or resource exposed via MCP JSON-RPC.
- **Don't use for:** standard HTTP REST endpoints (use `ENDPOINT--`).
- **Phase:** P2.

### `CMD--` · executable system command

- **Use for:** CLI tools, internal scripts, and background commands.
- **Don't use for:** multi-step workflows (use `RUNBOOK--`).
- **Phase:** P2.

### `SKILL--` · agent capability

- **Use for:** an action / tool the agent has been given access to.
- **Don't use for:** user-facing behaviour (use `FEAT--`) or modules (use `MOD--`).
- **Distinguishing question:** *who triggers it?* → if the agent triggers it from context, it's a SKILL.
- **Examples:** `SKILL--CHECK-DRIFT`, `SKILL--PROPOSE-INBOUND`, `SKILL--SUMMARISE-SESSION`.

### `PROTOCOL--` · interaction contract

- **Use for:** handshake / message-format contracts between agents or between agent and system (MCP, agent-to-agent).
- **Don't use for:** HTTP API endpoint contracts — those are `ENDPOINT--`.
- **Distinguishing question:** *is this a multi-step interaction?* → if yes, PROTOCOL; if request/response single-shot, ENDPOINT.

> ⚠️ **Disambiguation — `PROTOCOL--` ใช้ความหมาย CS standard** (interaction / communication contract — HTTP protocol, MCP protocol)
> **ไม่ใช่** "SOP / situational procedure" ตามภาษาทั่วไป สำหรับ "เมื่อ X เกิด ให้ทำ Y" ให้เลือก:
>
> - **`RUNBOOK--`** = on-call response guide ("ถ้าเห็น alert/incident → ทำ steps เหล่านี้")
> - **`GUARD--`** = runtime hard rule ("agent ห้ามทำ X โดยไม่มี Y") — renamed from `GUARDRAIL--` in v2.3
> - **`POLICY--`** = operational policy ("retain data 90 วัน", "rate limit 100rps")
> - **`SAFETY--`** = ethical / alignment rule (v2.3+) — distinct from `GUARD--` (structural)

### `GUARD--` · structural / behavioural guardrail (v2.3+)

- **Use for:** runtime-enforced constraint on agent / tool behaviour ("never call X without Y") OR a data-integrity invariant ("id must match filename", "no null primary keys").
- **Don't use for:** policy-by-decision (that's `ADR--`), operational policy (that's `POLICY--`), or ethical alignment (use `SAFETY--`).
- **Distinguishing question:** *is it enforced at every call / write?* → if yes, GUARD.
- **Renamed from:** `GUARDRAIL--` (pre-v2.3) — shortened for consistency with peer four-letter prefixes (`STACK`, `FRAME`, `SPEC`). Zero `GUARDRAIL--` atoms existed at migration time; the rename is doc-only.

### `POLICY--` · operational policy

- **Use for:** access policies (RBAC), data retention, rate limits.
- **Don't use for:** behavioural constraints during a single agent action — those are `GUARD--`.
- **Distinguishing question:** *does it govern config / access at the system level?* → POLICY.

### `PERSONA--` · agent identity

- **Use for:** the agent's role, voice, base system prompt seed.
- **Don't use for:** the agent's tools — those are `SKILL--`.

## Cluster 3 — Requirements Engineering

`CONCEPT--REQ.md` was previously the umbrella; that conflated
verification approaches. Split per ADR-012.

### `REQ--` · umbrella requirement

- **Use for:** cross-cutting requirements that span FR + NFR.
- **Optional:** projects that only have FR/NFR can skip the umbrella.

### `FR--` · functional requirement

- **Use for:** "system shall do X" — verifiable by unit / E2E.
- **Don't use for:** "system shall be fast / available / scalable" — those are `NFR--`.

### `NFR--` · non-functional requirement

- **Use for:** performance, scalability, security, availability, observability targets.
- **Verification:** load test / pen test / chaos test / availability monitoring.
- **Examples:** `NFR--P99-LATENCY-200MS`, `NFR--AVAILABILITY-99-9`.

### `CONSTRAINT--` · hard external constraint

- **Use for:** regulatory (GDPR, HIPAA, PCI), contractual, compliance.
- **Don't use for:** internally-chosen targets — those are `NFR--` or `POLICY--`.

## Cluster 4 — Ops Governance

The atomic surface of operations. Distinct from `MSP-INC-` etc., which
are process-tracking event logs.

### `INC--` · incident post-mortem

- **Use for:** distilled lesson from a production incident.
- **Don't use for:** the raw event log — that's `MSP-INC-` in process tracking.
- **Lifecycle:** written after triage; mostly stable thereafter.

### `HOTFIX--` · hotfix escape-hatch atom

- **Use for:** the 48-hour backfill window opened when prod is down and a fix
  ships before P1–P3 atoms exist (master-spec §6.4, ADR-014).
- **Don't use for:** the post-mortem itself — that's `INC--`.
- **Tier:** light — written automatically by `gks hotfix open` or the
  pre-commit hook when a `HOTFIX` tag is detected.
- **Required:** `valid_to` (= commit-time + 48 h) and `meta.commit_sha`.
- **Closure rule:** backfill atoms (`CONCEPT--`, `ADR--`, `BLUEPRINT--`) must
  declare `crosslinks.resolves: [HOTFIX--<sha>]`. After `valid_to`, the
  pre-commit hook blocks any commit on the affected files until that resolution
  is in place.

### `ISSUE--` · live issue tracker

- **Use for:** open problems / bugs / improvement requests — replaces Linear/Jira.
- **Don't use for:** decisions (use `ADR--`) or features (use `FEAT--`).
- **Lifecycle:** **mutates frequently** — status changes, comments, reassignments. Lives in the
  light-governance tier (`gks/issues/`); schema-validated but doesn't require human-review for
  routine operations. See ADR-012.
- **Required frontmatter:** `id`, `phase`, `type: issue`, `status` (open / triaged /
  in_progress / blocked / closed / wontfix), `priority` (low / medium / high / urgent),
  `created_at`, `updated_at`.
- **Recommended:** `assignee`, `reporter`, `labels`, `crosslinks.related_incidents`,
  `crosslinks.resolved_by`, `crosslinks.duplicates_of`, `crosslinks.blocks`.
- **Body convention:** `## Description`, `## Reproduction` (when applicable),
  `## Discussion` (append-only chronological), `## Resolution` (filled at close).

### `RISK--` · identified risk + mitigation

- **Use for:** "X could go wrong because Y" *before* it actually does.
- **Don't use for:** post-incident learning — that's `INC--`.

### `RUNBOOK--` · operational response guide

- **Use for:** "if you see X, do Y" — for on-call humans / agents.
- **Don't use for:** decisions (`ADR--`) or risk identification (`RISK--`).

### `SLO--` · service-level objective

- **Use for:** measurable availability / latency / error-rate targets + alert thresholds.
- **Don't use for:** non-measurable goals — those are `CONCEPT--` or `NFR--`.

## Memory-system extras

GKS code (`src/memory/types.ts` `AtomicType`) recognises three
additional types that are typically auto-generated by the Consolidator
rather than human-authored:

### `INSIGHT--` · session-derived observation

- Auto-extracted by `reflect()` from session traces — represents
  something noticed during a conversation that's worth retaining.

### `FACT--` · retain-derived fact

- Stored via `retain()` when the LLM asserts a discrete factual claim
  worth bi-temporal versioning.

### `RULE--` · derived behavioural rule

- A heuristic / pattern derived from multiple observations. Often
  promoted later into a formal `GUARD--` or `POLICY--` after review.

## Process tracking IDs (not atomic knowledge)

These have `MSP-` prefix and live in process-tracking storage, not in
`gks/`. Listed here for completeness — see FRAMEWORK_MASTER_SPEC §11.

| Prefix | Role |
|---|---|
| `MSP-SESS-` | Session ID |
| `MSP-IMP-` | Implementation Plan |
| `MSP-TSK-` | Task log |
| `MSP-ACT-` | Action log (per turn) |
| `MSP-WKT-` | Walkthrough (sign-off bundle) |
| `MSP-INC-` | Incident event (raw, distinct from `INC--` post-mortem) |
| `MSP-REV-` | Review |
| `MSP-FBK-` | Feedback |
| `MSP-USR-` | User identity |
| `MSP-AGT-` | Agent identity |

## Decision rule — "where does this concept go?"

```
        ↓ Is it about EXISTING code / decisions / features?
        │
        ├── Decision        → ADR--
        ├── Root policy     → MASTER--
        ├── Validator rule  → PROTO--
        ├── Feature spec    → FEAT--
        ├── Algorithm       → ALGO--
        ├── Architectural framework → FRAMEWORK--
        ├── Block manifest  → GENESIS-- (v2.3+: aggregates atoms into a Genesis Block)
        ├── Tech stack      → STACK--
        ├── Module           → MOD--
        ├── Data schema     → ENTITY--
        ├── Spec / contract → SPEC--
        ├── API contract    → API-- / ENDPOINT-- / ENTRYPOINT--
        ↓ Is it about WHAT TO BUILD?
        ├── Idea            → IDEA--
        ├── Concept / PRD   → CONCEPT--
        ├── Plan            → BLUEPRINT--
        ├── Microtask       → T*
        ├── Test result     → AUDIT--
        ↓ Is it about REQUIREMENTS?
        ├── Functional      → FR--
        ├── Non-functional  → NFR--
        ├── Hard constraint → CONSTRAINT--
        ↓ Is it about AGENT BEHAVIOUR?
        ├── Capability      → SKILL--
        ├── Interaction     → PROTOCOL--
        ├── Hard rule       → GUARD-- (renamed from GUARDRAIL-- in v2.3)
        ├── Ethical / safety → SAFETY--
        ├── Mental model    → COGNITIVE--
        ├── Operational     → POLICY--
        ├── Identity        → PERSONA--
        ↓ Is it about OPS?
        ├── Past incident   → INC--
        ├── Live problem    → ISSUE--
        ├── Future risk     → RISK--
        ├── Response guide  → RUNBOOK--
        ├── SLO target      → SLO--
        ↓ Auto-derived?
        └── INSIGHT-- · FACT-- · RULE--
```

## See also

- [`docs/adr/012-extended-taxonomy.md`](./adr/012-extended-taxonomy.md) — decision record
- [`examples/atom-templates/`](../examples/atom-templates/) — starter `.md` templates per prefix
- [`docs/MSP_RELATIONSHIP.md`](./MSP_RELATIONSHIP.md) — how MSP gates these atom writes
- `FRAMEWORK_MASTER_SPEC §4.1` — meta-architecture summary (defers to this file as canonical)
