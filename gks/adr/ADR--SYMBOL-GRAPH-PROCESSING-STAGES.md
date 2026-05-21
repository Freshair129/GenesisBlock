---
id: ADR--SYMBOL-GRAPH-PROCESSING-STAGES
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Decision on 12-Stage Symbol Graph Processing Pipeline
tags: &a1
  - msp
  - gks
  - adr
  - graph-processing
crosslinks: &a2
  references:
    - CONCEPT--SYMBOL-GRAPH-PIPELINE
    - FRAMEWORK--SYMBOL-GRAPH
created_at: 2026-05-13T19:46:00.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--SYMBOL-GRAPH-PROCESSING-STAGES
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Decision on 12-Stage Symbol Graph Processing Pipeline
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T19:46:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--SYMBOL-GRAPH-PROCESSING-STAGES
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Decision on 12-Stage Symbol Graph Processing Pipeline
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T19:46:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--SYMBOL-GRAPH-PROCESSING-STAGES
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Decision on 12-Stage Symbol Graph Processing Pipeline
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-13T19:46:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — Symbol Graph Processing Stages

> Alias: this ADR formalises the stage breakdown of **Block Decomposition** — the top-down half of the Genesis Block Cycle. See `docs/gks/[[PRD--GENESIS-BLOCK-CYCLE]].md` for the unified vocabulary.

## Context

ในการสร้าง Symbol Graph ที่มีความหมายเชิงสถาปัตยกรรม (Architectural Meaning) การสกัดเพียงแค่ AST (Abstract Syntax Tree) ไม่เพียงพอต่อการทำความเข้าใจความสัมพันธ์ที่ซับซ้อน เช่น ลำดับชั้นการสืบทอด, ระบบ Routing ของ Framework, และการไหลของข้อมูลระหว่างไฟล์

## Decision

เราตัดสินใจบังคับใช้กระบวนการประมวลผล 12 ระยะ (12-stage DAG) โดยจัดลำดับความสำคัญตามขั้นตอนการขยายความรู้ (Knowledge Expansion):

1. **Physical Discovery:** (Scan, Structure) - เข้าใจพื้นที่ทางกายภาพ
2. **Specialized Extraction:** (Markdown, COBOL) - ดึงข้อมูลเฉพาะทางและ Legacy
3. **Symbolic Extraction:** (Symbolic Parse) - ดึงโครงสร้างโค้ดดิบ (Raw Symbols)
4. **Framework Intelligence:** (Routes, Tools, ORM) - เพิ่มบริบทของเทคโนโลยีที่ใช้
5. **Relationship Resolution:** (Cross-File, MRO) - เชื่อมโยงสัญลักษณ์เข้าด้วยกัน
6. **Abstract Analysis:** (Communities, Processes) - สรุปความหมายในระดับสูง

## Consequences

- **Positive:** Agents สามารถมองเห็นภาพรวมของระบบ (Big Picture) ได้ทันทีผ่าน Graph DB
- **Positive:** รองรับการทำ Impact Analysis ข้ามภาษาและข้ามระบบ (Legacy + Modern)
- **Negative:** ใช้ทรัพยากรในการประมวลผลครั้งแรกสูง (Initial Indexing Cost) แต่จะคุ้มค่าในการ Query ครั้งต่อๆ ไป
- **Positive:** ข้อมูลจาก Stage 12 (Processes) เป็นปัจจัยหลักที่ป้อนเข้าสู่ **MLL (Meta Learning Loop)** เพื่อใช้ในการสร้าง Skill อัตโนมัติและตรวจจับ Tension
- **Constraint:** ต้องรันผ่าน `GenesisGraphBackend` เพื่อประสิทธิภาพในการจัดการความสัมพันธ์เชิงลึก

## Connections

- [[CONCEPT--SYMBOL-GRAPH-PIPELINE]]
- [[FRAMEWORK--SYMBOL-GRAPH]]
