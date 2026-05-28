---
id: CONCEPT--HOP-BASED-RESOLUTION
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: "Hop-Based Resolution — การนึกแบบมนุษย์ผ่านความลึกของ Graph Chain"
tags:
  - msp
  - ucf
  - concept
  - retrieval
  - resolution
  - graph
  - recall
  - human-cognition
crosslinks:
  expands_on:
    - CONCEPT--RESOLUTION-GRADIENT
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - FEAT--RESOLUTION-EXPAND-ON-DEMAND
    - FEAT--HIERARCHICAL-RECALL
    - FRAMEWORK--CROSSLINKS-VOCABULARY
created_at: 2026-05-29T02:41:57.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: knowledge-engine
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Hop-Based Resolution

> ระบบการนึกแบบมนุษย์ — เมื่อเราคิดถึงสิ่งหนึ่ง (Node D) เราไม่ได้นึกถึงทุกสิ่งที่เกี่ยวข้องพร้อมกัน เราเห็นสิ่งที่ใกล้ที่สุดอย่างชัดเจน สิ่งที่ไกลออกไปจะเลือนลง และสิ่งที่ปลายสุดเราแค่รู้ว่ามันมีอยู่ — เหมือนแสงไฟฉายที่ส่องจุดศูนย์กลางสว่างจ้า แล้วค่อย ๆ มืดลงไปรอบ ๆ
>
> ขยายจาก `[[CONCEPT--RESOLUTION-GRADIENT]]` ที่กำหนด 4 ระดับ (FULL / SUMMARY / SKELETON / MENTION) โดยเพิ่มกลไก **Graph Hop Distance** เป็นตัวกำหนดว่าแต่ละ Node จะถูกจัดอยู่ที่ Resolution Tier ใด

## Problem

`[[CONCEPT--RESOLUTION-GRADIENT]]` กำหนด **Resolution Tiers** ไว้แล้ว (FULL / SUMMARY / SKELETON / MENTION) แต่ยังขาดกลไกการตัดสินใจว่า **แต่ละ Node ควรได้ Tier ใด** เมื่อเดินตาม Chain ของ Crosslinks

ระบบค้นหาปัจจุบันใช้สูตร `score = w1·similarity + w2·1/(1+hops)` ซึ่งเป็นการนับ Hop เบื้องต้น แต่ยังไม่ได้แปลค่า Hop Distance เป็น Resolution Tier อย่างเป็นระบบ

ปัญหาหลัก: Agent ที่ค้นหา Node D ใน Chain จะได้:
- **ปัจจุบัน**: ได้ทุก Node ในระดับ FULL หรือ MENTION เท่านั้น (กระโดดจากเห็นหมดไปเห็นแค่ชื่อ ไม่มีระดับกลาง)
- **ที่ต้องการ**: ได้ข้อมูลตามระยะทาง — ใกล้เห็นชัด ไกลเห็นเลือน

## Hypothesis

การใช้ **Graph Hop Distance** จากจุดค้นหา (Search Origin) เป็นตัวกำหนด Resolution Tier ของแต่ละ Node ใน Chain จะทำให้ Agent ได้ข้อมูลที่ใกล้เคียงกับวิธีการนึกของมนุษย์มากขึ้น

### กลไกการทำงาน

```
Chain: /| Y~4 ←→ Z~3 ←→ A~2 ←→ (B ←→ {C ←→ [D] ←→ E} ←→ F) |\

ค้นหา D (Search Origin):

  Hop 0 → [D]    → FULL     เห็นทุกอย่าง (body + frontmatter + crosslinks)
  Hop 1 → {C, E} → SUMMARY  เห็น summary, anchor, salient, trigger, hook
  Hop 2 → (B, F) → SKELETON เห็นแค่ title, anchor, salient, trigger, hook
  Hop 3+ → Node~hop (เช่น A~2, Z~3, Y~4) → MENTION เห็นแค่ id และจำนวน hop

สัญลักษณ์และระบบคำบรรยาย Chain (Hybrid Notation):

  [ Node ]      Focus Node / Parent (Hop 0) — แสดงแบบ FULL
  { Node }      Child / Near Node (Hop 1)   — แสดงแบบ SUMMARY
  ( Node )      Grandchild / Far Node (Hop 2) — แสดงแบบ SKELETON (ยังมี chain ต่อ)
  Node~hop      Mention / Deep Node (Hop 3+) — แสดงแบบ MENTION โดยมีเลข hop กำกับ (เช่น Node~3, Node~4)

สัญลักษณ์แสดงทิศทางสุดทาง (End of Chain / Terminal Node):

  /|            สุดสายสัมพันธ์ด้านซ้าย (Left Terminal Node)
  |\            สุดสายสัมพันธ์ด้านขวา (Right Terminal Node)
  /| |\         สุดสายสัมพันธ์ทั้งซ้ายและขวา (เช่นมีโหนดเดียวโดดๆ หรือสายเดี่ยวที่จบสมบูรณ์)
  
สัญลักษณ์สายเชื่อมสัมพันธ์ (Edge / Relationships):

  ←             ความสัมพันธ์ทิศทางเดียว (ชี้กลับฝั่งซ้าย)
  →             ความสัมพันธ์ทิศทางเดียว (ชี้ไปฝั่งขวา)
  ←→            ความสัมพันธ์สองทิศทาง (Bidirectional Link)
```

### สัญลักษณ์แสดง Chain Boundary

```
สัญลักษณ์      ความหมาย
────────────────────────────────────────────────────────────
[ X ]         Focus Node (Hop 0) — FULL Detail
{ X }         Near Node (Hop 1) — SUMMARY
( X )         Far Node (Hop 2) — SKELETON (ยังมี chain ไปต่อ)
X~hop / | X | End of Chain / Terminal Node — MENTION (สุดทาง หรือ อยู่ระดับลึก Hop 3+)
```

### Anchor Fields (ฟิลด์สมอที่ปรากฏในทุก Tier)

แม้ Node จะอยู่ไกลจาก Focus (Hop 2+) ก็ยังคงเห็น "สมอ" (Anchor Fields) เพื่อให้ Agent ตัดสินใจได้ว่าจะ `expand()` หรือไม่:

| Field | คำอธิบาย | แสดงใน Tier |
|:---|:---|:---|
| `id` | Compound ID ของ Atom | ทุก Tier |
| `title` | หัวข้อย่อ | SKELETON ขึ้นไป |
| `summary` | คำอธิบายสั้น ≤250 ตัวอักษร | SUMMARY ขึ้นไป |
| `salient` | จุดเด่นที่สำคัญที่สุดของ Node | SKELETON ขึ้นไป |
| `trigger` | เงื่อนไขที่ทำให้ Node นี้มีความเกี่ยวข้อง | SKELETON ขึ้นไป |
| `hook` | จุดเชื่อมโยงกลับไปยัง Focus Node | SKELETON ขึ้นไป |

### Configurable Hop-to-Tier Mapping

ค่าเริ่มต้นของ Hop → Tier สามารถปรับได้ตาม Vault หรือ Task Context:

```yaml
# PARAMS--HOP-RESOLUTION-DEFAULT
hop_tier_map:
  0: FULL        # Focus node — เห็นทุกอย่าง
  1: SUMMARY     # เพื่อนบ้าน — เห็น summary + anchor fields
  2: SKELETON    # ห่างออกไป — เห็น title + anchor fields
  3: MENTION     # สุดทาง — เห็นแค่ id
max_depth: 3     # หยุดเดิน graph เมื่อถึง hop 3
```

ตัวอย่างการปรับ: Agent ที่ทำ Deep Research อาจใช้ `max_depth: 5` และ `hop_tier_map: {0: FULL, 1: FULL, 2: SUMMARY, 3: SKELETON, 4: MENTION}` เพื่อเห็นรายละเอียดลึกขึ้น

## Scope

In:

- **Hop Counter**: อัลกอริทึมเดิน BFS/DFS บน Genesis Graph ออกจาก Focus Node แล้วนับ hop distance
- **Hop → Tier Mapper**: ฟังก์ชันแปลง hop distance เป็น Resolution Tier ตาม configurable map
- **Anchor Field Extraction**: ระบบดึงฟิลด์สมอ (title, summary, salient, trigger, hook) ตาม Tier
- **Bidirectional Walk**: เดิน graph ทั้งสองทิศทาง (upstream + downstream ของ crosslinks)
- **Chain Boundary Detection**: ตรวจจับว่า node เป็น leaf (สุดทาง) หรือยังมี chain ต่อ
- **Configurable `max_depth`**: จำกัดความลึกสูงสุดของการเดิน graph
- **Integration กับ `expand()`**: Agent สามารถ `expand()` Node จาก SKELETON → FULL ได้ตามปกติ

Out:

- **Vector Similarity Scoring**: ระบบ vector search ยังคงทำงานตามเดิม — Hop-Based Resolution ทำงาน **หลังจาก** Layer 3 (Graph + Vector Scoring) ใน UCF Pipeline
- **ABAC Policy**: ระบบสิทธิ์ยังคงทำงานก่อน Tier Assignment — Node ที่ถูก deny จะไม่ปรากฏในผลลัพธ์
- **Adaptive Learning**: การเรียนรู้ว่า Agent expand บ่อยแค่ไหน → ปรับ hop_tier_map อัตโนมัติ (Phase 5+)
- **Cross-vault Chain Walk**: การเดิน graph ข้าม vault — ควบคุมโดย `[[FEAT--VAULT-COMPOSITION]]`

## Crosslinks Predicates ที่สร้าง Chain ได้

จาก `[[FRAMEWORK--CROSSLINKS-VOCABULARY]]` คีย์ต่อไปนี้จะถูกใช้เป็น **edge** ในการเดิน graph:

| Predicate | ทิศทาง | การเดิน Graph | ความหมายทาง Chain |
|:---|:---|:---|:---|
| `depends_on` | this → parent | Upstream walk | ขึ้นไปหา Parent (ผู้ถูกพึ่งพา) |
| `belongs_to` | this → container | Upstream walk | ขึ้นไปหา Module/Group แม่ |
| `implements` | this → spec | Upstream walk | ขึ้นไปหา Spec/Blueprint ต้นทาง |
| `references` | this → context | Both | เชื่อมโยงบริบท (อ่อนที่สุด) |
| `expands_on` | this → base | Upstream walk | ขยายจากแนวคิดเดิม |
| `supersedes` | this → old | Upstream walk | แทนที่เวอร์ชันเก่า |

### Edge Weight (น้ำหนักของขอบ)

ไม่ใช่ทุก crosslink จะมีน้ำหนักเท่ากัน:

```
Predicate        Weight    เหตุผล
──────────────────────────────────────────────
depends_on       1.0       ความสัมพันธ์ที่แน่นที่สุด (Parent-Child)
belongs_to       1.0       ความสัมพันธ์ Module-Feature
implements       0.9       Spec → Implementation
expands_on       0.8       ขยายจากแนวคิดเดิม
references       0.5       บริบทอ่อน — อาจต่อ chain ได้ แต่ห่างออกไป
supersedes       0.3       แทนที่ — แค่ประวัติศาสตร์
```

## ตัวอย่างการทำงานจริง

### Case: ค้นหา RUNBOOK--OBSIDIAN-LINTER-SETUP

```
Hop 0 [FULL]:
  RUNBOOK--OBSIDIAN-LINTER-SETUP
  ├── depends_on → Hop 1
  └── references → Hop 1

Hop 1 [SUMMARY]:
  PARAMS--LINTER-SETUP-PROFILE01   (via depends_on, weight 1.0)
  CONCEPT--OBSIDIAN-AS-RUNTIME     (via references, weight 0.5)

Hop 2 [SKELETON]:
  RUNBOOK--COVIBE-BACKLOG-SYNC     (via PARAMS used_by, weight 0.5)
  ADR--MSP-OBSIDIAN-INTEGRATION    (via CONCEPT references, weight 0.25)

Hop 3 [MENTION]:
  FRAMEWORK--MSP-ARCHITECTURE-V2 |\ (สุดทาง chain)
```

Agent จะเห็น:
```
📄 RUNBOOK--OBSIDIAN-LINTER-SETUP           ← FULL (เห็นทุกอย่าง)
  📋 PARAMS--LINTER-SETUP-PROFILE01         ← SUMMARY (เห็นตาราง + สรุป)
  📋 CONCEPT--OBSIDIAN-AS-RUNTIME           ← SUMMARY (เห็นสรุปย่อ)
    📌 ADR--MSP-OBSIDIAN-INTEGRATION        ← SKELETON (เห็นแค่หัวข้อ)
      🔗 FRAMEWORK--MSP-ARCHITECTURE-V2 |\  ← MENTION (รู้ว่ามี แต่ไม่เห็น - สุดทาง)
```

## Verification

- BFS จาก Node ใดก็ตาม → ได้ Node ทั้งหมดใน chain พร้อม hop distance ที่ถูกต้อง
- Node ที่ hop 0 ได้ FULL, hop 1 ได้ SUMMARY, hop 2 ได้ SKELETON, hop 3+ ได้ MENTION
- `expand()` บน Node ที่เป็น SKELETON → ได้ FULL กลับมาและนับ token ได้
- Chain boundary (| สัญลักษณ์สุดทาง) ถูกตรวจจับเมื่อ Node ไม่มี outgoing edge เพิ่มเติม
- Token consumption ลดลง ≥60% เทียบกับการ FULL ทุก Node ใน chain
- `max_depth` ถูกเคารพ — ไม่เดิน graph เลย hop 3 (หรือค่าที่ตั้งไว้)

## Source

- `[[CONCEPT--RESOLUTION-GRADIENT]]` — กำหนด 4-tier model (FULL/SUMMARY/SKELETON/MENTION)
- `[[FEAT--RESOLUTION-EXPAND-ON-DEMAND]]` — expand() API contract
- `[[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]` — UCF 5-layer pipeline, Layer 4 = Resolution tier
- `[[FEAT--HIERARCHICAL-RECALL]]` — Multi-tier RRF fusion
- `[[FRAMEWORK--CROSSLINKS-VOCABULARY]]` — Predicate definitions
- HippoRAG (Gutiérrez et al., 2024) — prior art: multi-hop retrieval over knowledge graphs
- Self-RAG (Asai et al., 2023) — prior art: agent-directed retrieval

## Connections

- [[CONCEPT--RESOLUTION-GRADIENT]]
- [[FEAT--RESOLUTION-EXPAND-ON-DEMAND]]
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
- [[FEAT--HIERARCHICAL-RECALL]]
- [[FRAMEWORK--CROSSLINKS-VOCABULARY]]
