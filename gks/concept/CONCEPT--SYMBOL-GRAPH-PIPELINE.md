---
id: CONCEPT--SYMBOL-GRAPH-PIPELINE
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Symbol Graph Processing Pipeline — 12-Stage DAG
tags: &a1
  - msp
  - gks
  - symbol-graph
  - pipeline
  - dag
crosslinks: &a2
  references:
    - CONCEPT--SYMBOL-GRAPH
    - FRAMEWORK--SYMBOL-GRAPH
    - SPEC--GENESIS-GRAPH-BACKEND
created_at: 2026-05-13T19:45:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--SYMBOL-GRAPH-PIPELINE
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Symbol Graph Processing Pipeline — 12-Stage DAG
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T19:45:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--SYMBOL-GRAPH-PIPELINE
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Symbol Graph Processing Pipeline — 12-Stage DAG
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T19:45:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--SYMBOL-GRAPH-PIPELINE
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Symbol Graph Processing Pipeline — 12-Stage DAG
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-13T19:45:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Symbol Graph Processing Pipeline

> Alias: **Block Decomposition** — the top-down half of the Genesis Block Cycle (the bottom-up `Block Assembly` half is governed by `[[FRAMEWORK--PHASE-GOVERNANCE]]`). See `docs/gks/[[PRD--GENESIS-BLOCK-CYCLE]].md` for the unified vocabulary.

แนวคิดหลักคือการเปลี่ยน **Source Code (Unstructured Data)** ให้กลายเป็น **Architectural Knowledge Graph (Structured Knowledge)** ผ่านกระบวนการประมวลผลที่เป็นลำดับขั้นตอน (Deterministic Pipeline) ในรูปแบบของ **Directed Acyclic Graph (DAG)**

## The 12-Stage DAG

ระบบจะรันงานผ่าน 12 ระยะหลักเพื่อให้ได้ข้อมูลที่สมบูรณ์ที่สุด:

1. **Scan (การสแกน):** สำรวจ File paths และสถิติพื้นฐานของ Repository
2. **Structure (โครงสร้าง):** สร้าง Hierarchy Tree ของโฟลเดอร์และไฟล์
3. **Specialized Parse - Markdown:** สกัดความรู้จากเอกสารเพื่อเชื่อมโยงเนื้อหาอธิบาย (Atoms) เข้ากับโค้ด
4. **Specialized Parse - COBOL:** รองรับการวิเคราะห์ระบบ Legacy ข้ามยุคสมัย
5. **Symbolic Parse:** สกัดสัญลักษณ์ (Functions, Classes, Methods) โดยใช้ **Tree-sitter AST**
6. **Framework - Routes:** วิเคราะห์ API Entry Points (เช่น Next.js App Router)
7. **Framework - Tools:** สกัดนิยาม MCP tools และ RPC handlers
8. **Framework - ORM:** วิเคราะห์การจัดการข้อมูลและความสัมพันธ์ Database (Prisma/Supabase)
9. **Cross-File Resolution:** เชื่อมโยง Imports/Exports ระหว่างไฟล์
10. **MRO (Method Resolution Order):** วิเคราะห์ลำดับการสืบทอด (Heritage Map)
11. **Communities (การจัดกลุ่มชุมชน):** ใช้ **Leiden Algorithm** จัดกลุ่มสัญลักษณ์ตามหน้าที่ (Functional Cohesion)
12. **Processes (กระบวนการประมวลผล):** ติดตามเส้นทางการไหลของข้อมูล (Execution Flows) — **เป็น Input หลักให้ MLL Skill Creator**

## Destination: GenesisGraphBackend

ผลลัพธ์สุดท้ายจะถูกบันทึกลงใน **GenesisGraphBackend** เพื่อให้ Agents สามารถ Query ความสัมพันธ์เชิงลึกได้ทันที โดยไม่ต้องประมวลผลใหม่ในทุกครั้งที่ใช้งาน

## Connections

- [[CONCEPT--SYMBOL-GRAPH]]
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[SPEC--GENESIS-GRAPH-BACKEND]]
