---
id: GENESIS--COGNITIVE-SYSTEM-ROOT
phase: 0
type: genesis
status: stable
vault_id: GKS-CORE
tier: master
source_type: axiomatic
title: Genesis Master Manifest — Cognitive System Architectural Root
tags:
  - genesis
  - manifest
  - roadmap
  - architecture
  - entry-point
cluster: implementation_flow
role: Genesis Block Manifest (Runtime Entry-point)
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
    - ADR--MONOREPO-STRUCTURE
    - ADR--AGENT-WRITE-BOUNDARIES
    - SPEC--ATOM-REGISTRY-SCHEMA
    - COGNITIVE--EGO-DEATH-PASSPORT
    - ALGO--IDENTITY-RESOLUTION
    - RUNBOOK--IDENTITY-MIGRATION
    - PARAMS--RETRIEVAL-WEIGHTS
attributes:
  manifest_version: 1.0.0
  domain: genesis
  daci:
    driver:       MOD--COGNITIVE
    approver:     Boss
    contributor:  [QWEN-T1, CLAUDE-T3]
    informed:     [ANTIGRAVITY]
created_at: 2026-05-28T14:30:00+07:00
promoted_from: CONCEPT--TIERED-MEMORY-DISTILLATION
promoted_at: 2026-05-28T14:30:00.000+07:00
promotion_adr: ADR--MONOREPO-STRUCTURE
---

# GENESIS — Cognitive System Master Manifest

เอกสารฉบับนี้คือ **Runtime Entry-point** สำหรับระบบ Cognitive System ทำหน้าที่เป็นแผนที่นำทาง (Map of Maps) เพื่อเชื่อมโยงและสรุปสาระสำคัญของสถาปัตยกรรมหลัก โดยไม่ต้องไล่อ่านเอกสารต้นฉบับทั้งหมด

## 1. 🏗️ Foundation & Architecture (รากฐาน)

| Atom ID | Path | สรุปสาระสำคัญ (Short Summary) |
| :--- | :--- | :--- |
| `FRAMEWORK_MASTER_SPEC` | [[FRAMEWORK_MASTER_SPEC]] | กฎเหล็กของระบบ: Doc-Before-Code และ 3-Tier Knowledge (GKS/MSP/Obsidian) |
| `ADR--MONOREPO-STRUCTURE` | `gks/adr/ADR--MONOREPO-STRUCTURE.md` | การแบ่ง Layer: `packages/gks` (Storage) และ `packages/msp` (Orchestrator) |
| `ADR--AGENT-WRITE-BOUNDARIES` | `gks/adr/ADR--AGENT-WRITE-BOUNDARIES.md` | ขอบเขตสิทธิ์: Agent เขียนได้เฉพาะ `.brain/` (Candidate); ส่วน `gks/` ต้องผ่าน PR เท่านั้น |

## 2. 🧠 Memory & Distillation (ระบบความจำ 8-8-8)

| Atom ID | Path | สรุปสาระสำคัญ (Short Summary) |
| :--- | :--- | :--- |
| `CONCEPT--TIERED-MEMORY-DISTILLATION` | `gks/concept/CONCEPT--TIERED-MEMORY-DISTILLATION.md` | ลำดับการกลั่นกรองความจำ: Session (ชั่วคราว) -> Episodic (สำคัญ) -> Identity (ตัวตน) |
| `ADR--CONSOLIDATOR-HYBRID-SCORING` | `gks/adr/ADR--CONSOLIDATOR-HYBRID-SCORING.md` | วิธีเลือกสิ่งที่ควรจำ: ใช้ Deterministic Rules (80%) + LLM Scoring (20%) |
| `ADR--EMBEDDING-MODEL-PARITY` | `gks/adr/ADR--EMBEDDING-MODEL-PARITY.md` | มาตรฐาน Semantic Search: ใช้ `nomic-embed-text-v1.5` ทั้งระบบ (Headless & UI) |

## 3. 🛡️ Governance & Security (การควบคุมและนโยบาย)

| Atom ID | Path | สรุปสาระสำคัญ (Short Summary) |
| :--- | :--- | :--- |
| `SPEC--ATOM-REGISTRY-SCHEMA` | `gks/spec/SPEC--ATOM-REGISTRY-SCHEMA.md` | SSOT ของ Taxonomy: กำหนด ID Prefix, ประเภทไฟล์ และโฟลเดอร์ที่จัดเก็บ |
| `ADR--ANTI-HALLUCINATION-RULES` | `gks/adr/ADR--ANTI-HALLUCINATION-RULES.md` | กฎเหล็ก 6 ข้อในการเขียน Atom: ห้าม Link เสีย, ห้ามปลอมแปลง ID และฟิลด์สถานะ |
| `FEAT--POLICY-DECISION-POINT` | `gks/feat/FEAT--POLICY-DECISION-POINT.md` | ระบบ UCF: ควบคุมการเข้าถึง (Auth/ABAC) แบบ Transport-Agnostic |

## 4. 🛠️ Codegen & Execution (การผลิตและรันโค้ด)

| Atom ID | Path | สรุปสาระสำคัญ (Short Summary) |
| :--- | :--- | :--- |
| `CONCEPT--CODEGEN-MICROTASK-RUNNER` | `gks/concept/CONCEPT--CODEGEN-MICROTASK-RUNNER.md` | ระบบผลิตโค้ดอัตโนมัติ: แบ่ง Blueprint เป็น T* Micro-tasks เพื่อส่งให้ SLM รัน |
| `ADR--CODEGEN-RETRY-POLICY` | `gks/adr/ADR--CODEGEN-RETRY-POLICY.md` | นโยบายการแก้บั๊ก: Retry 3 ครั้งบน SLM -> Escalate ไป Gemini -> จบที่ Human Review |

---

## 🧭 How to navigate
1.  **สำหรับ Agent:** สแกน `gks/00_index/atomic_index.jsonl` เพื่อหาความสัมพันธ์เชิงลึก
2.  **สำหรับ Human:** ใช้เอกสารนี้เป็นสารบัญ (MOC) และเปิดอ่านรายละเอียดผ่าน ID ใน Obsidian
3.  **Upstream:** ทุกการเปลี่ยนแปลงต้องอ้างอิงกลับมาที่ `GENESIS--` หากกระทบโครงสร้างหลัก
