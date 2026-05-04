# MSP — Memory & Soul Passport — Technical Full Specification

> **Version:** 2.0.1
> **Status:** Draft (architecture v2 + GKS audit alignment)
> **Audience:** T3 Architects, T2 Implementers, MSP maintainers
> **Authority:** เมื่อขัดแย้งกับเอกสารนี้ ให้ยึด `gks/frame/FRAME--MSP-ARCHITECTURE-V2.md` เป็นหลัก

> **เปลี่ยนจาก v1.0.0 → 2.0.0** (ดู §17): MSP ถูก reframe จาก "gatekeeper" เป็น **passport** ที่พกพา memory + soul + retrieval + identity ของ agent ไปทุกที่. Gatekeeping ยังเป็นหน้าที่หนึ่ง แต่ไม่ใช่ identity. GKS = canonical knowledge layer (atomic markdown), Obsidian = runtime, Smart Connections = embedding source
>
> **เปลี่ยนจาก 2.0.0 → 2.0.1** (M7-prep follow-up): GKS audit อัปเดต — GKS ตอนนี้เป็น canonical embedder (`createNomicEmbedder()` ใน 3.6.0), Smart Connections เป็น in-Obsidian browse path; ทั้งสอง lock ที่ `nomic-embed-text-v1.5`. `OBSIDIAN_HOST` → `OBSIDIAN_URL`. Atomic graph เป็น GKS scope (MSP shift-left validation only). 4 upstream proposals สำหรับ GKS

---

## 0. TL;DR

**MSP** คือ **passport** ที่เดินทางไปกับ agent — ห้องสมุดความจำ + ตัวตน + ตรรกะการเรียกคืนความรู้

| สิ่งที่ MSP "พก" ติดตัว agent ไป | Module |
|---|---|
| Sessions (ทุก turn ของบทสนทนา) | `src/memory/sessions/` |
| Episodic memory (สิ่งที่สำคัญในบทสนทนา) | `src/memory/episodic/` |
| Consolidator (importance scoring + summarisation) | `src/orchestrator/consolidator.ts` (M7b) |
| Retrieval orchestration (`msp_recall` fuse Obsidian + episodic) | `src/orchestrator/retrieval.ts` (M7c) |
| Context compression (token-budget aware) | `src/orchestrator/compressor.ts` (M7d) |
| Identity / soul (agent profile, voice, preferences) | `src/identity/` (M7e) |
| GKS write-path validator (gatekeeper for atom writes) | `src/validator/` |
| Codegen runner (T*.task.yaml → src/) | `src/codegen/` |
| MCP tool surface | `src/mcp/` |

**GKS** (Genesis Knowledge System) คือ **canonical knowledge layer** — atom markdown + wikilinks/crosslinks/backlinks. ไม่มี runtime ของตัวเอง

**Obsidian** = runtime ของ GKS — vault watcher, search, graph view, Local REST API, plugins. MSP delegate search/graph ไปที่ Obsidian เพื่อไม่ build ทับ

**Smart Connections** = embedding source (local, GUI-resourced). MSP **ไม่เคย embed เอง**

**Gatekeeper aspect** (เดิมเป็น focus หลักของ v1) ยังอยู่ครบ:

1. ผ่าน **inbound queue** (`/submit-memory` → `.brain/msp/projects/<ns>/inbound/`)
2. ผ่าน **validator** (frontmatter schema + forbidden fields + ID format + wikilink resolution + 6 anti-hallucination rules)
3. ผ่าน **human review** (per `ADR--HUMAN-REVIEW-GATES`)
4. ถูก **promoted** โดย MSP เป็นคนเซ็ตฟิลด์ derived

ทางลัด: `HOTFIX` tag ที่ต้อง backfill P1–P3 ภายใน 48 ชั่วโมง (`ADR--HOTFIX-ESCAPE-HATCH`)

---

## 1. ปัญหาที่ MSP แก้ (Why)

MSP มีอยู่เพื่อกัน 4 ความเสี่ยงเฉพาะของ multi-agent + LLM-driven knowledge base:

| ความเสี่ยง | ตัวอย่าง | MSP กันยังไง |
|---|---|---|
| **SSOT corruption** | agent เขียนทับ ADR ที่ approved แล้วด้วยข้อมูลผิด | inbound queue ไม่อนุญาตเขียน `gks/adrs/` ตรง ๆ |
| **ID collision** | agent ตั้ง ADR-079 ทั้ง ๆ ที่มีอยู่แล้ว | rule `adr_id_must_be_max_plus_one` |
| **Frontmatter hallucination** | agent แต่งฟิลด์ `validated_by`, `commit_hash` เอง | `forbidden_fields` ใน contract |
| **Dangling wikilink** | `[[FEAT--xxx]]` ชี้ไปไฟล์ที่ไม่มี | rule `no_dangling_wikilinks` (severity: error) |

MSP ไม่ได้ทำหน้าที่ชดเชยทุกข้อบกพร่องของ agent — มันคือ **schema enforcement + ID discipline + link integrity** เท่านั้น คุณภาพเชิงเนื้อหา (เช่น ADR เขียนได้ดีไหม) ยังต้องผ่าน human review

---

## 2. Architecture Overview

### 2.0 Two-layer mental model (v2)

```
                    Agent (Claude / Cursor / EVA / custom)
                            │
                            ▼
                ┌────────────────────────┐
                │     MSP (passport)     │
                │  travels with agent    │
                │                        │
                │  - sessions            │
                │  - episodic memory     │
                │  - consolidator (M7b)  │
                │  - retrieval orch (M7c)│
                │  - context compress(M7d│
                │  - identity / soul(M7e)│
                │  - validator (gks adp.)│
                │  - codegen runner      │
                └───────────┬────────────┘
                            │ knowledge queries
                            ▼
                ┌────────────────────────┐
                │   GKS (knowledge)      │
                │  atomic .md + wikilinks│
                │  + atomic_index.jsonl  │
                │  + backlinks.jsonl     │
                └───────────┬────────────┘
                            │ runtime
                            ▼
                ┌────────────────────────┐
                │   Obsidian (vault)     │
                │  - file watching       │
                │  - text search         │
                │  - graph view          │
                │  - Local REST API      │
                │  - Smart Connections   │
                │    (local embeddings)  │
                │  - obsidian-mcp        │
                └────────────────────────┘
                            │ scale-up later
                            ▼
                  [vector DB / graph DB]
```

อ้างอิง: `gks/frame/FRAME--MSP-ARCHITECTURE-V2.md`

### 2.1 Write flow (gatekeeper aspect of MSP)

```
┌─────────────────────────────────────────────────────────────┐
│  Agent (T1/T2/T3)                                           │
│  - draft .md หรือ .yaml ใน scratchpad                       │
└─────────────┬───────────────────────────────────────────────┘
              │ /submit-memory  หรือ  msp_propose tool
              ▼
┌─────────────────────────────────────────────────────────────┐
│  .brain/msp/projects/<ns>/inbound/                          │
│  - {ID}.rev-{reviewId}.md                                   │
└─────────────┬───────────────────────────────────────────────┘
              │ npm run msp:validate (or pre-commit hook)
              ▼
┌─────────────────────────────────────────────────────────────┐
│  MSP Validator                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ 1. required_fields (per type, from atomic_contract)   │  │
│  │ 2. forbidden_fields (17 banned keys)                  │  │
│  │ 3. ID format (TYPE--SLUG uppercase + ADR-NNN + HOTFIX)│  │
│  │ 4. ID-filename match                                  │  │
│  │ 5. ADR monotonic (max+1)                              │  │
│  │ 6. Dangling wikilinks (body + crosslinks)             │  │
│  │ 7. Future date guard                                  │  │
│  │ 8. Summary min/max + placeholder ban                  │  │
│  │ 9. Phase ↔ status compat                              │  │
│  │ 10. No invented versions (semver only)                │  │
│  │ 11. Evidence for decisions (ADR sections)             │  │
│  │ — soft warn: cite-or-mark-inferred                    │  │
│  └───────────────────────────────────────────────────────┘  │
└─────┬──────────────────────────────────────┬────────────────┘
   pass│                                  fail│
      ▼                                       ▼
┌─────────────────────┐               ┌──────────────────────┐
│ Human Review        │               │ rejected/{date}/     │
│ (per ADR--HUMAN-    │               │ + rejection_reason   │
│  REVIEW-GATES)      │               │                      │
└────────┬────────────┘               └──────────────────────┘
         │ approve
         ▼
┌─────────────────────────────────────────────────────────────┐
│  npm run msp:promote (gks inbound promote)                  │
│  - move file → gks/<type>/                                  │
│  - set status = stable                                      │
│  - re-index (atomic_index.jsonl + backlinks.jsonl)          │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Read flow (passport aspect of MSP — M7+)

```
Agent → msp_recall(query)
        │
        ▼
   retrieval orch.
        ├── Obsidian REST text search (keyword via Local REST API)
        ├── Smart Connections semantic search (if Obsidian + plugin live)
        ├── episodic memory read (MSP-owned)
        └── backlinks.jsonl traversal (graph hop, MSP-owned)
        │
        ▼
   RRF merge → ranked top-K → returned with provenance
```

### 2.3 Write flow for memory (passport aspect — M7+)

```
Agent → msp_remember(turn) → sessions writer (append JSONL)
                          ↓ if importance threshold met
                       consolidator → episodic writer (append episode)
                          ↓ if context overflow
                       compressor → summarise old episodes → re-write
```

### 2.4 Layers

| Layer | Path | Owner | บทบาท |
|---|---|---|---|
| **Contract** | `.brain/msp/LLM_Contract/` | MSP | YAML schema ที่ validator อ่าน |
| **Inbound** | `.brain/msp/projects/evaAI/inbound/` | Agent (write), MSP (read) | queue ของ proposal |
| **Rejected** | `.brain/msp/projects/evaAI/rejected/{date}/` | MSP | proposal ที่ fail พร้อม reason |
| **Sessions** | `.brain/msp/projects/evaAI/sessions/*.jsonl` | MSP | linear conversation history |
| **Episodic Memory** | `.brain/msp/projects/evaAI/memory/episodic_memory.json` | MSP | rich event summaries |
| **Vector / Backlinks** | `.brain/msp/projects/evaAI/vector/` | MSP | hybrid retrieval support |
| **Scripts** | `scripts/msp/*.mjs` | Maintainer | CLI runner ของ MSP |
| **Project rules** | `msp/rules/`, `msp/LLM_Contract/` | T3 Architect | กฎเพิ่มเติมเฉพาะโปรเจกต์ |

---

## 3. Inbound Flow

### 3.1 Submission Envelope

ทุก proposal ที่ agent submit ต้องมี **envelope frontmatter** ครอบไฟล์จริง:

```yaml
---
proposal_id: "20260502-093000-claude-new_atomic-msp-techspec"
proposal_type: new_atomic     # new_atomic | update_atomic | supersede | deprecate
target_file: "gks/adrs/ADR-080.md"
submitted_by: "@claude-opus-4-7"
submitted_at: "2026-05-02T09:30:00Z"
rationale: "Document MSP technical surface for new joiners"
---
# (atomic body — ไฟล์ atomic frontmatter + content เริ่มที่นี่)
```

**Filename pattern:** `{yyyymmddHHMMSS}-{agent_id}-{proposal_type}-{slug}.md`
**Inbound path:** `.brain/msp/projects/evaAI/inbound/`

### 3.2 Proposal Types

| Type | ทำหน้าที่ | Side effect ตอน promote |
|---|---|---|
| `new_atomic` | สร้าง atomic ใหม่ใน `gks/<type>/` | ID ต้องไม่ซ้ำ |
| `update_atomic` | แก้ atomic ที่มีอยู่ | `version` ต้อง increment ตาม semver |
| `supersede` | ทำเครื่องหมาย atomic เก่าว่า superseded + link ใหม่ | เก่าไม่ลบ, set `status: superseded` + `superseded_by` |
| `deprecate` | mark เป็น deprecated | ต้องระบุ `valid_until` |

---

## 4. Atomic Write Contract

อ้างอิง: `.brain/msp/LLM_Contract/phase2_atomic_contract.yaml`

### 4.1 Required Fields (ทุก atomic)

```yaml
required_fields:
  - id          # เช่น CONCEPT--POS-SYSTEM, ADR-079
  - phase       # 0–6
  - type        # idea | concept | algorithm | entity | framework | feature | module | protocol | flow | architecture_decision
  - status      # stub | raw | draft | active | stable | deprecated | superseded | partial
  - vault_id    # เช่น POS-001
  - summary     # ≤ 300 chars, ≥ 10 chars, single sentence
  - created_at  # ISO-8601 (YYYY-MM-DD)
  - created_by  # @agent-id หรือ @human-handle
```

### 4.2 Conditional Required

| Field | Condition |
|---|---|
| `version` | `type ∈ {module, feature, protocol}` |
| `module` | `type ∈ {feature, flow}` |
| `depends_on` | `type == feature` |
| `context_anchor.duration` | `type ∈ {architecture_decision, protocol}` |
| `epistemic.confidence` | `type ∈ {architecture_decision, protocol}` |

### 4.3 Forbidden Fields (agent ห้ามตั้งเอง)

```yaml
# Identity forgery guard
- commit_hash, merge_commit, tenant_id, pr_number, reviewer_approved_at

# Authority fields (MSP only)
- promotion_level, validated_at, validated_by, msp_signature, hash

# Runtime-only metrics
- execution_count, last_error, uptime, latency_p50

# Fabrication risk
- adr_number_override, feature_id_override, incident_id
```

ถ้าตรวจเจอฟิลด์เหล่านี้ใน inbound → **proposal rejected** ทันที

### 4.4 Field Constraints

| Field | Constraint |
|---|---|
| `id` | regex `^(ADR-[0-9]{3}\|FEAT--[a-z0-9-]+\|MOD--[a-z0-9-]+\|PROTO--[a-z0-9-]+\|FLOW--[a-z0-9-]+)$` + ต้องตรงกับ filename |
| `type` | enum (architecture_decision, feature, module, protocol, flow) |
| `status` | enum (draft, active, deprecated, superseded, partial, stable, raw, stub) |
| `summary` | min 10 / max 300 chars; forbid `TBD`, `TODO`, `FIXME`, `lorem ipsum` |
| `depends_on` | `wikilink_array` — ทุก link ต้อง resolve ได้ |
| `epistemic.confidence` | number 0.0–1.0 |
| `epistemic.source_type` | enum (direct_experience, documented_source, inferred, hypothesis, external, axiom) |

### 4.5 Anti-Hallucination Rules

| Rule | Severity | คำอธิบาย |
|---|---|---|
| `no_dangling_wikilinks` | error | `[[link]]` ทุกอันต้องมี target จริง |
| `no_invented_adr_numbers` | error | ADR ใหม่ต้อง = max(existing) + 1; agent ต้อง read `atomic_index.jsonl` ก่อน |
| `no_invented_versions` | error | semver-only; first draft = 0.1.0 |
| `evidence_for_decisions` | error | ADR ต้องมี Context + Decision + Consequences |
| `cite_or_mark_inferred` | warning | claim เกี่ยวกับโค้ด (path/line/function) ต้องอ้าง blueprint หรือ tag `inferred` + `confidence < 1.0` |
| `no_future_dates` | error | ห้ามตั้ง `created_at` ในอนาคต |

### 4.6 Epistemic & Crosslinks Block

```yaml
epistemic:
  confidence: 0.0–1.0
  source_type: direct_experience | documented_source | inferred | external | axiom
  duration: permanent | temporary | deprecated   # temporary → ต้องระบุ valid_until

crosslinks:
  implements: []     # ADR/กฎหมายที่ atomic นี้ไปรองรับ
  used_by: []        # ใครพึ่งพา component นี้
  references: []     # บริบทอื่น
  contradicts: []    # link ไป atomic ที่ขัดแย้ง
```

---

## 5. Codegen Micro-task Contract (Phase 3.5)

อ้างอิง: `.brain/msp/LLM_Contract/codegen_microtask_contract.yaml`

ใช้กับเฉพาะ **SLM output** (Qwen 2.5 Coder, Llama local) ที่รัน `T*.task.yaml` ใน `gks/microtasks/`

### 5.1 Post-processing (ก่อน acceptance test)

- `strip_markdown_fences: true` — เอา ` ``` ` ออก
- `strip_leading_commentary: true` — ตัดบรรทัดก่อน `export`/`import`/`const`
- `strip_trailing_commentary: true` — ตัดบรรทัดหลัง `}` ตัวสุดท้าย
- `normalize_line_endings: "\n"`

### 5.2 Forbidden Imports

**ถ้าไม่อยู่ใน `package.json`:** `joi, zod, yup, ajv, uuid, lodash, ramda, axios, moment, underscore, bluebird, request`
**ห้ามทุกกรณี:** `fs, child_process, net, http, "../"` (ใช้ `@/` alias)

### 5.3 Forbidden Patterns

| Pattern | Severity | เหตุผล |
|---|---|---|
| `export default` | error | Next.js App Router ใช้ named exports |
| `req.body` | error | App Router ใช้ `await req.json()` |
| `req\.tenantId` | error | tenantId มาจาก `withAuth` context, ไม่ใช่ req |
| `console\.(log\|debug\|info\|error\|warn)` | warning | route handler ห้าม log ตรง — ใช้ Sentry/Pino |
| `process\.env\.` | warning | route-level env access เป็น smell — ใช้ `@/lib/config` |
| `// TODO\|// FIXME\|// XXX` | error | SLM ห้าม punt — retry แทน |

### 5.4 Required Patterns ราย slot

| Slot | Must contain |
|---|---|
| `exports` | `export const POST =` |
| `handler` | `export async function` หรือ `export const.*= async` |
| `helpers` | เริ่มด้วย `export function` หรือ `export const` |

### 5.5 Retry Policy

```yaml
max_retries: 3
include_failed_test_in_next_prompt: true
include_forbidden_pattern_match: true
strip_previous_attempt_from_ctx: true   # fresh ทุก retry
```

ถ้าครบ 3 retry แล้วยัง fail → **escalate ไป Gemini** (`scripts/msp/escalate-to-gemini.mjs {task_id}`); ถ้า Gemini ยัง fail → fallback เป็น Opus review

---

## 6. Phase Governance — The Ruler of Flow

อ้างอิง: `FRAMEWORK_MASTER_SPEC.md` §7.7

MSP ไม่ใช่แค่ schema validator — มันคุม **phase gating** ด้วย ก่อน agent จะเข้า P5 (เขียนโค้ดใน `src/`) ต้องผ่าน checklist ตาม Scaling Level

### 6.1 Scaling Level → Required Artifacts

| Scale | Use case | Artifacts ขั้นต่ำ |
|---|---|---|
| **L1** | Quick task, 1 concern | `MSP-ACT-` + `MSP-WKT-` |
| **L2** | Feature/module ทั่วไป | `CONCEPT--` + `ADR--` + **`API--`** + `T*` task + `MSP-WKT-` |
| **L3** | Major / core / critical | `PRD` + `REQ` + `ADR--` + `FLOW--` + **`API--`** + `BLUEPRINT--` + `AUDIT--` + `MSP-WKT-` |

### 6.2 Phase 1 Technical Feasibility

P1 บังคับให้ `CONCEPT--` มี **High-level API Draft** (รายการ endpoint สำคัญ) — ป้องกันสถานการณ์ที่ business approve แต่ตอน P2 พบว่า technical ทำไม่ได้

### 6.3 Mandatory OpenAPI ที่ P2

P2 ต้องแตก draft เป็น 3 atomic:
- **`API--`** — Master Hub (OpenAPI spec อยู่ที่นี่)
- **`ENDPOINT--`** — 1 path/method ต่อไฟล์
- **`ENTRYPOINT--`** — auth/middleware logic

### 6.4 Devlog Tracking

| Phase | Atomic Type | Devlog ID |
|---|---|---|
| P3 (Plan) | `BLUEPRINT--` | `MSP-IMP-` |
| P4 (Decompose) | `T*.task.yaml` | `MSP-TSK-` |
| P5 (Code) | `src/` | `MSP-ACT-` (per turn) |
| P6 (Audit) | `AUDIT--` | `MSP-WKT-` (walkthrough handover) |

ทุก devlog entry **ต้องมี `sessionId`** เพื่อ traceability

---

## 7. Memory Subsystem

> หลักการ: **Memory for Audit, ไม่ใช่ Full Context Reload** — ห้าม agent ย้อนอ่านทั้ง history เพื่อกัน token explosion เข้าถึงแบบ selective ผ่าน ID อ้างอิง

### 7.1 Linear Session History (JSONL)

**Path:** `.brain/msp/projects/evaAI/sessions/<episodicId>.jsonl`
**ใช้:** ไล่เรียงเหตุการณ์ราย turn

```json
{
  "sessionId": "<string>",
  "episodicId": "<string>",
  "turnId": <number>,
  "msgId": "<string>",
  "speakerId": "user | MSP-AGT-XXX",
  "content": "<string>",
  "learnId": "<knowledge-id-if-any>"
}
```

### 7.2 Rich Episodic Memory (JSON)

**Path:** `.brain/msp/projects/evaAI/memory/episodic_memory.json`
**ใช้:** สรุป episode สำคัญพร้อม metadata

```json
{
  "episodicId": "ep_001",
  "sessionId": "sess_001",
  "projectId": "evaAI",
  "timestamp": "2026-04-18T10:30:00Z",
  "importance_score": 0.85,
  "range": ["turnIdx-x"],
  "anchor": { "content": "...", "msgId": "..." },
  "context": { "topic": "...", "participants": [], "mood": "..." },
  "content": {
    "summary": "...",
    "key_decisions": [],
    "unresolved_questions": []
  },
  "tags": [],
  "associations": {
    "related_event_ids": [],
    "entity_links": [],
    "knowledgeId": "FEAT--xxx",
    "learnId": "ALGO--xxx"
  }
}
```

### 7.3 Vector / Backlinks

**Path:** `.brain/msp/projects/evaAI/vector/backlinks.jsonl`
แต่ละ edge: `{ from: "ID-A", to: "ID-B", type: "implements|used_by|..." }`
ใช้สำหรับ hybrid retrieval (RRF) ตาม `FRAMEWORK_MASTER_SPEC.md` §13

---

## 7a. Obsidian as Runtime *(v2 — updated 2.0.1)*

อ้างอิง: `gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md` + `gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md` + `gks/adr/ADR--GRAPH-IS-GKS-DOMAIN.md`

> **Updated 2.0.1 (M7-prep follow-up)**: env var renamed `OBSIDIAN_HOST` → `OBSIDIAN_URL` to match GksV3 3.6.0's `RestObsidianAdapter`. M7a now wraps GKS's existing adapter rather than building fresh. Atomic graph (wikilinks/backlinks) is GKS scope per `ADR--GRAPH-IS-GKS-DOMAIN`; MSP only does shift-left validation.

### 7a.1 ทำไม Obsidian

Atom files ใน `gks/` คือ markdown + YAML frontmatter + `[[wikilink]]` — รูปแบบที่ Obsidian optimise มา. ถ้า user's `gks/` เป็น Obsidian vault ด้วย → Obsidian ให้ฟรี:

| ฟรีจาก Obsidian | แทนที่อะไรใน MSP |
|---|---|
| Text + tag + frontmatter search | grep / custom search |
| Backlinks pane + graph view | manual graph traversal |
| File watching (re-index on save) | inotify wrapper |
| Local REST API plugin (HTTP-queryable) | REST server ใน MSP |
| Smart Connections plugin (local embeddings) | bundled embedder |
| obsidian-mcp (MCP server) | additional MCP server |
| UI สำหรับ human browse | MSP web UI |

MSP **delegate** ทุกอย่างนี้ไป Obsidian ผ่าน `ADR--MSP-OBSIDIAN-INTEGRATION` (wraps GKS `RestObsidianAdapter`):

- **Primary**: Obsidian Local REST API (`OBSIDIAN_URL` env, default `https://127.0.0.1:27124`)
- **Fallback**: filesystem (read `gks/<type>/*.md`, `atomic_index.jsonl`, `backlinks.jsonl`)
- **Note**: agent-facing semantic recall ใช้ GKS embedder ตรง — ไม่ผ่าน Obsidian (เปลี่ยนจาก v2.0.0)

### 7a.2 Authentication + TLS

- API key อ่านจาก `OBSIDIAN_API_KEY` env หรือ `~/.config/msp/obsidian.key`
- ถ้าไม่มี key → skip REST + ไป filesystem fallback (ไม่ error spam)
- TLS bypass อนุญาตเฉพาะ `127.0.0.1`/`localhost` (self-signed cert ของ plugin); `OBSIDIAN_INSECURE=true` ให้ override สำหรับ local dev
- Env vars: `OBSIDIAN_URL` (default `https://127.0.0.1:27124`), `OBSIDIAN_API_KEY`, `OBSIDIAN_INSECURE`. `OBSIDIAN_HOST` (จาก v2.0.0 draft) deprecated; M7a อ่านเป็น fallback หนึ่ง minor release พร้อม warning

### 7a.3 Detection

`createObsidianClient(opts)` (M7a wrapper รอบ GKS adapter) คืน `mode: 'rest' | 'filesystem'`. Caller เช็ค `mode === 'rest'` ก่อนขอ feature ที่ต้องการ live Obsidian (เช่น Smart View deep links). **Semantic recall ไม่ขึ้นกับ `mode`** — GKS embedder ทำงานทั้งสอง mode

---

## 7b. Embedding Strategy *(v2 — updated 2.0.1)*

อ้างอิง: `gks/concept/CONCEPT--EMBEDDING-STRATEGY.md` + `gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md` + `gks/adr/ADR--EMBEDDING-MODEL-PARITY.md`

> **Updated 2.0.1 (M7-prep follow-up)**: original 2.0.0 said "MSP ไม่เคย embed; Smart Connections เป็น canonical". GksV3 3.6.0 ships `createNomicEmbedder()` — GKS เป็น canonical embedder ตอนนี้. Reframed: GKS = canonical writer (agent path), Smart Connections = in-Obsidian human browse. ทั้งสอง surface ใช้ **model เดียวกัน** lock โดย `ADR--EMBEDDING-MODEL-PARITY`.

### 7b.1 หลักการ (v2 updated)

**Two surfaces, one model**:

| Surface | Owner | Used by | Storage |
|---|---|---|---|
| Agent-facing semantic recall | **GKS** (`createNomicEmbedder()` + `VectorBackend`) | `msp_recall` (M7c), MCP tools, headless CI | `.brain/.../vector/atomic.jsonl` (or HNSW / pgvector) |
| Human browse in Obsidian | **Smart Connections** plugin | Smart View pane, "find similar notes" | `.smart-connections/` (plugin-private) |

Canonical model: **`nomic-ai/nomic-embed-text-v1.5`** (768-dim, Thai+English mixed-content). User ต้อง configure Smart Connections ให้ตรงกัน (Obsidian Settings → Smart Connections → Embedding Model).

### 7b.2 Constraints ที่เกิดจากการเลือกนี้

1. **Agent path ไม่ต้อง running Obsidian** — GKS vector store ทำงาน headless ได้ (CI, server boot, no GUI). เปลี่ยนจาก v2 ที่บอกว่าต้องการ Obsidian
2. **MSP compute query embedding ได้** — ผ่าน GKS adapter (`createNomicEmbedder`); ไม่ผ่าน Smart Connections
3. **`.smart-connections/` schema ยังไม่ stable** — MSP ห้ามอ่าน vectors ตรง (เป็น diagnostic only เหมือนเดิม)
4. **2× storage cost** จนกว่า M10a "msp-bridge" plugin จะลง — ยอมรับได้ที่ vault < 5,000 atoms

### 7b.3 Integration mechanism

**Agent path (canonical)**:
- M7a wraps GKS's `createNomicEmbedder()` + `VectorBackend`. ไม่ network hop, ไม่ Obsidian dependency

**Human path (browse)**:
- Smart Connections plugin in Obsidian, configured ให้ใช้ canonical model
- M7a probe REST + plugin availability; surface "open in Smart View" deep links เมื่อมี

### 7b.4 Scale-up path

- โครงการเล็ก/กลาง: GKS JSONL + Smart Connections พอ
- โครงการใหญ่: swap GKS `VectorBackend` → HNSW หรือ pgvector. MSP ไม่ต้องแก้
- M10a: companion plugin ให้ Smart Connections อ่าน GKS vector store ตรง — single index

### 7b.5 What if user picks a different model in Smart Connections

Allowed but documented as deviation. Agents ยังได้ผลถูกต้อง (GKS canonical); แค่ Smart View pane ใน Obsidian จะแสดง neighbours ที่ไม่ตรงกับ agent's view. **Drift visible to humans, not destructive**.

---

## 7c. Retrieval Orchestration *(v2 — M7c)*

`msp_recall(query)` คือ tool ที่ fuse ข้อมูลจากหลาย sources:

```
msp_recall(query)
  ├── GKS vector store            (semantic; canonical — works headless)
  ├── Obsidian REST text search   (keyword; if mode === 'rest', else grep fallback)
  ├── episodic memory             (MSP-owned; matches against summary + tags)
  └── backlinks.jsonl traversal   (graph hop from query-matched atoms)
       │
       ▼
   RRF (Reciprocal Rank Fusion) merge
       │
       ▼
   Ranked top-K with provenance:
   {
     hits: [
       { atom_id, source: 'gks-vector' | 'obsidian-text' | 'grep' | 'episodic' | 'backlinks',
         score, snippet }, ...
     ],
     semantic_available: bool,                    // GKS path is always available
     obsidian_available: bool,                    // for Smart View deep-link affordance
     fallback_reason?: string,
   }
```

หลัก: caller ไม่ต้องรู้ว่า hit มาจาก source ไหน — orchestration ฟิวให้แล้ว. provenance อยู่ในผลลัพธ์เผื่อ debugging

---

## 7d. Context Compression *(v2 — M7d)*

ขณะ context window ใกล้เต็ม:

```
compressor.compress({ episodes, budget_tokens })
  → กลุ่ม episodes ตาม importance_score
  → top-K importance ปล่อยเต็ม
  → ส่วนที่เหลือ summarise ผ่าน LLM (configurable provider)
  → คืน compressed list + provenance (ของเก่าอยู่ที่ไหน)
```

หลัก: compression **lossy แต่ traceable** — ทุก compressed group บอกได้ว่ามาจาก episodes ไหน

---

## 7e. Identity / Soul *(v2 — M7e)*

`src/identity/` เก็บสิ่งที่ทำให้ "agent นี้คือ agent นี้" ข้าม sessions:

- **Profile** — ชื่อ, role, tier (T1/T2/T3)
- **Voice** — tone, formality, language preference
- **Preferences** — favoured tools, default flags, aliases
- **Operational state** — last session id, ongoing context pointer

เก็บที่ `.brain/msp/projects/<ns>/identity.json` (atomic write). MCP tools `msp_identity_get` / `msp_identity_set` ให้ agent อ่าน/เขียนตัวเอง

---

## 8. Promotion & Rollback

### 8.1 Promotion Levels

| Level | สถานะ |
|---|---|
| **L0** | อยู่ใน inbound, ยังไม่ validate |
| **L1** | ผ่าน schema + anti-hallucination แล้ว |
| **L2** | human-reviewed + merged เข้า `gks/<type>/` |

### 8.2 Promotion Command

```bash
npm run msp:promote
# - select file (number) หรือ 0 = all
# - move .brain/msp/projects/evaAI/inbound/{file}
#   → gks/<type>/{file}
# - set status = stable (และฟิลด์ derived อื่น ๆ)
```

`scripts/msp/promote.mjs` รู้จัก type prefix → folder mapping:

```js
ADR → gks/adrs       ALGO → gks/algorithms     API → gks/apis
CONCEPT → gks/concepts   ENTITY → gks/entities    FEAT → gks/features
FLOW → gks/flows     FRAME → gks/frameworks    IDEA → gks/ideas
BLUEPRINT → gks/blueprints   MOD → gks/modules    PARAMS → gks/parameters
```

### 8.3 Rollback

| Trigger | Action |
|---|---|
| validator fails | move file → `.brain/msp/projects/evaAI/rejected/{YYYY-MM-DD}/` + create `rejection_reason.md` |
| reviewer rejects | เหมือนกัน + ระบุ reviewer + reason |

---

## 9. Human Review Gates

| Artifact | Reviewer | Action |
|---|---|---|
| New ADR | Boss | approve / reject |
| FEAT spec | Boss | approve for Phase 2 |
| Blueprint (P3) | Boss | approve for Phase 3.5 |
| Task YAML | Implementer (self) + runner tests | gate by acceptance test |
| Generated code | CI + PR review | merge to branch |

---

## 10. Escape Hatches

### 10.1 Hotfix

```yaml
hotfix:
  allowed: true
  requires_tag: "HOTFIX"
  backfill_deadline_hours: 48
```

ถ้าฉุกเฉิน CI พัง → commit ได้พร้อม tag `HOTFIX` แต่ต้อง backfill `ADR--` / `BLUEPRINT--` ภายใน 48 ชั่วโมง ไม่งั้น CI fail รอบหน้า

### 10.2 Legacy Files

```yaml
legacy_files:
  allowed: true
  requires_frontmatter: "legacy: true"
```

ไฟล์ก่อนยุคมี contract → exempt จาก strict validation แต่ถูก flag ใน report

---

## 11. Tooling — `scripts/msp/`

### 11.1 CLI Commands

| Command | Script | บทบาท |
|---|---|---|
| `npm run msp:propose` | `propose.mjs` | wizard สร้าง atomic draft → `.msp/inbound/` |
| `npm run msp:review` | `review.mjs` | human ดู queue + diff |
| `npm run msp:promote` | `promote.mjs` | ย้าย inbound → `gks/<type>/` |
| `npm run msp:validate` | `validate.mjs` | full audit GKS (entries + edges) |
| `npm run msp:index` | `index.mjs` + `re-indexer.mjs` | rebuild `atomic_index.jsonl` + backlinks |
| (auto) | `pre-commit-validator.mjs` | block commit ถ้า MSP fail |
| (P6) | `write-wkt.mjs` | สร้าง walkthrough log |
| (changelog) | `append-changelog.mjs`, `global-changelog.mjs` | sliding window changelog |

### 11.2 Validator Library — `scripts/msp/lib/`

- **`validator.mjs`** — `validateEntry(entry, allIds)` (phase↔status, ID format), `validateEdges(edges, allIds)` (dead link)
- **`crosslink.mjs`** — derive backlinks
- **`review-state.mjs`** — track inbound queue state

### 11.3 Pre-commit Hook

```bash
# .git/hooks/pre-commit
node scripts/msp/pre-commit-validator.mjs
```

ตอนนี้ implementation ยังเป็น placeholder (เช็คแค่ index file มี) — TODO: เช็ค staged files vs registry rules

---

## 12. Project Path Encoding

MSP รองรับหลายโปรเจกต์ใต้ `~/.brain/msp/projects/<name>/` ใช้ **bare name** (เช่น `evaAI`) เป็น canonical encoding

> 📌 Decision recorded in `ADR--PATH-ENCODING`: ใช้ bare name ตาม `gks init` / `scripts/migration/standardizer.mjs` ไม่ใช้ `D--<name>` prefix ที่ spec ฉบับก่อนระบุไว้ — เพราะ tooling เป็น authoritative และ migration cost ของ rename ทุกโปรเจกต์สูงกว่าการแก้ spec หนึ่งย่อหน้า

---

## 13. Identity / Authority Matrix

| Path | Direct write | Channel |
|---|---|---|
| `gks/adrs/`, `gks/algorithms/`, `gks/entities/`, `gks/features/`, `gks/flows/`, `gks/frameworks/`, `gks/modules/`, `gks/parameters/`, `gks/concepts/`, `gks/ideas/` | ❌ | `/submit-memory` → inbound |
| `gks/blueprints/` | ✅ (T3 only — Claude) | direct edit, human review required |
| `gks/microtasks/` | ✅ (T2/T3) | acceptance tests gate execution |
| `gks/14_devlog/` | ✅ (free-write) | log per session |
| `src/` (auto-generated) | ❌ | edit task YAML + rerun codegen |
| `CLAUDE.md`, `GEMINI.md`, `registry.yaml` | ❌ (Boss-only) | ask first |
| `.brain/msp/projects/evaAI/inbound/` | ✅ (agents) | drop proposal |
| `.brain/msp/LLM_Contract/` | ❌ (MSP maintainer only) | code review |

---

## 14. Failure Modes & Diagnostics

| Symptom | สาเหตุ | แก้ |
|---|---|---|
| `❌ Atomic Index not found` | ลืมรัน re-indexer | `npm run msp:index` |
| `Dead link: X -> Y` | wikilink ชี้ atomic ที่ยังไม่ promote | promote target ก่อน หรือลบ link |
| `Phase 1 cannot be 'implemented'` | สถานะไม่ตรงกับ phase | แก้ frontmatter |
| `Invalid ID format` | ผิด pattern `TYPE--NAME` หรือ `ADR-NNN` | rename ตาม regex |
| `forbidden field: commit_hash` | agent หลงเขียน derived field | ลบฟิลด์ ออก |
| ADR number ชน | ไม่ได้ check `atomic_index.jsonl` ก่อน | ใช้ max(existing)+1 |
| SLM output มี `console.log` | ไม่ผ่าน codegen contract | retry หรือ escalate ไป T2 |

---

## 15. Open Issues / TODO

(ตาม CLAUDE.md "Known repo state quirks" + scan `scripts/msp/`)

- `pre-commit-validator.mjs` — implementation ยังเป็น placeholder
- `system_config.yaml` — `roles:` และ `audit.actions:` ยังว่าง
- `gks/concepts/FEATxx-*.md` (POS, CRM, Kitchen ฯลฯ) เป็น legacy domain — ต้อง confirm กับ Boss ก่อนลบ
- Path encoding `D--evaAI` vs `evaAI` — เลือกอันใดอันหนึ่งและ harmonize
- `validator.mjs` — ยังไม่ enforce ทุก rule ใน `phase2_atomic_contract.yaml` (เช่น forbidden_fields, anti-hallucination บางตัว)

---

## 16. References

| Doc | บทบาท |
|---|---|
| `FRAMEWORK_MASTER_SPEC.md` §7 | authoritative source ของ MSP gatekeeper |
| `FRAMEWORK_MASTER_SPEC.md` §13 | hybrid retrieval (4-layer RRF) |
| `.brain/msp/LLM_Contract/phase2_atomic_contract.yaml` | atomic write contract |
| `.brain/msp/LLM_Contract/codegen_microtask_contract.yaml` | SLM codegen contract |
| `Metadata Standard.md` | frontmatter spec |
| `gks/00_index/agent-protocol.md` | full reading/writing protocol |
| `CLAUDE.md` | T3 agent instructions (Claude/Opus) |
| `GEMINI.md` | T2 agent instructions (Gemini) |

---

## 17. Changelog

| Version | Date | Author | Note |
|---|---|---|---|
| 1.0.0 | 2026-05-02 | @claude-opus-4-7 | Initial extraction from `FRAMEWORK_MASTER_SPEC.md` §7 + scripts/msp + LLM_Contract |
| 2.0.0 | 2026-05-03 | @claude-opus-4-7 | Architecture v2 — passport over Obsidian-backed GKS. Reframed §0 TL;DR + §2 Architecture; added §7a Obsidian as Runtime, §7b Embedding Strategy (Smart Connections), §7c Retrieval Orchestration (M7c), §7d Context Compression (M7d), §7e Identity / Soul (M7e). Driving atoms: `FRAME--MSP-ARCHITECTURE-V2`, `CONCEPT--OBSIDIAN-AS-RUNTIME`, `CONCEPT--EMBEDDING-STRATEGY`, `ADR--MSP-OBSIDIAN-INTEGRATION`, `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`. v1.0.0 sections 3–6, 8–13 unchanged (gatekeeper aspects still authoritative). |
| 2.0.1 | 2026-05-04 | @claude-opus-4-7 | M7-prep follow-up — GKS audit alignment. §7a env var rename `OBSIDIAN_HOST` → `OBSIDIAN_URL` to match GksV3 3.6.0 `RestObsidianAdapter`. §7b reframed: GKS is canonical embedder (`createNomicEmbedder()` ships in 3.6.0), Smart Connections is in-Obsidian browse path; both lock to `nomic-embed-text-v1.5`. §7c retrieval source list updated (GKS vector store primary). New atoms: `ADR--GRAPH-IS-GKS-DOMAIN`, `ADR--EMBEDDING-MODEL-PARITY`. Updated atoms: `CONCEPT--EMBEDDING-STRATEGY`, `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`, `ADR--MSP-OBSIDIAN-INTEGRATION`, `CONCEPT--MEMORY-VECTOR-BACKLINKS`, `ADR--ANTI-HALLUCINATION-RULES` (shift-left clarification). 4 upstream proposals drafted under `upstream/gks-proposals/`. See `AUDIT--M7-PREP-FOLLOWUP`. |