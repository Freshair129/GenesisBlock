---
id: FEAT--GENESISDB-HIGH-PERFORMANCE-ENGINE
phase: 2
type: feat
status: stable
vault_id: GKS-CORE
tier: process
source_type: learned
title: "Feature: GenesisDB — Ultra-Low-Latency Axiomatic Graph Engine"
tags: [gks, feat, genesisdb, graph-engine, rust]
aliases: [genesisdb-feature]
created_at: 2026-05-30T05:00:00+07:00
crosslinks:
  references: [CONCEPT--GENESIS-GRAPH-BACKEND, GENESIS--BACKEND-ENGINE]
  refined_by: [ADR--GENESISDB-CSR-MUTATION-STRATEGY, ADR--GENESISDB-GOVERNANCE-LOGIC, ADR--GENESISDB-TEMPORAL-MODEL]
  implemented_by: [ALGO--KIMPACT-CALCULATION]
attributes:
  domain: backend-storage
  p95_latency: <1ms
---

# FEAT--GENESISDB-HIGH-PERFORMANCE-ENGINE

## 1. Executive Summary
ฟีเจอร์นี้มอบเครื่องยนต์ฐานข้อมูลกราฟแบบฝัง (Embedded) ประสิทธิภาพสูงที่พัฒนาด้วยภาษา Rust เพื่อทำหน้าที่เป็น "Long-term Memory" สำหรับ AI Agents ระบบถูกออกแบบมาเพื่อแก้ปัญหาคอขวดของฐานข้อมูลกราฟแบบ Client-Server ทั่วไป โดยเน้นที่ความเร็วในการสืบค้นระดับ Microsecond และการบังคับใช้กฎเกณฑ์ความถูกต้องของข้อมูล (Axiomatic Safety) ในระดับ Storage Layer

## 2. User/Agent Benefits
*   **Real-time Reasoning:** AI Agent สามารถเดินกราฟเพื่อหาความสัมพันธ์เชื่อมโยง 3 ชั้น (3-hops) ได้ในเวลาไม่ถึง 1 มิลลิวินาที ทำให้กระบวนการ Chain-of-Thought ไม่สะดุด
*   **Axiomatic Safety:** ป้องกัน Agent จากการแก้ไขข้อมูลพื้นฐาน (Master Rules) โดยไม่ตั้งใจผ่านกฎ Tier-based Protection
*   **Perfect Recall:** รองรับการทำ Time-travel Query เพื่อดูสถานะของความรู้ ณ จุดเวลาใดก็ได้ในอดีต (Reproducible Reasoning)
*   **Zero-Maintenance:** รันแบบ In-process ไม่ต้องติดตั้งหรือจัดการ Database Server ภายนอก

## 3. High-Level API Contract (Functional)
เครื่องยนต์ต้องสนับสนุน Interface หลักดังนี้:
*   `neighbors(node_id, depth)`: ดึงโหนดเพื่อนบ้านในระดับลึกพร้อมจัดลำดับตาม [[ALGO--KIMPACT-CALCULATION]]
*   `add_edge(src, tgt, rel, supersede_flag)`: สร้างความสัมพันธ์พร้อมตรวจสอบ [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
*   `query(cypher_subset)`: รองรับการสืบค้นรูปแบบ Cypher สำหรับโครงสร้างที่ซับซ้อน
*   `compact()`: รวบรวมและบีบอัดข้อมูลลงดิสก์แบบ Atomic ตาม [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]]

## 4. Acceptance Criteria
*   [x] ผ่านการทดสอบ **LDBC-Lite Benchmark** (3-hop < 1ms)
*   [x] ผ่านการทดสอบ **Axiomatic Guard** (Low-tier cannot overwrite High-tier)
*   [x] ผ่านการตรวจสอบ **Atomic Integrity** (Snapshot ไม่เสียหายเมื่อ Crash)
*   [x] มีเอกสารประกอบระดับ **Systems Engineering** ครบถ้วน

---
### Related Links
- **Root Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **Storage Decision:** [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]]
- **Governance Decision:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
