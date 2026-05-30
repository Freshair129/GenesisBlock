---
id: SPEC--GENESISDB-FFI-CONTRACT
phase: 3
type: spec
status: candidate
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: "Specification: GenesisDB FFI Interface and Technical Contract"
tags: [gks, spec, ffi, rust, interface]
created_at: 2026-05-30T05:00:00+07:00
crosslinks:
  references: [FEAT--GENESISDB-HIGH-PERFORMANCE-ENGINE]
attributes:
  domain: backend-storage
  runtime: rust-native
---

# SPEC--GENESISDB-FFI-CONTRACT

## 1. Introduction
เอกสารนี้ระบุข้อกำหนดทางเทคนิคและ Interface การเชื่อมต่อ (FFI) สำหรับเครื่องยนต์ GenesisDB เพื่อให้แน่ใจว่าการสื่อสารระหว่างชั้น Application (Node.js/Python) และ Storage Engine (Rust) เป็นไปตามประสิทธิภาพที่คาดหวัง

## 2. API Interface Contract
เครื่องยนต์ต้องสนับสนุน Interface หลักดังนี้:
*   `neighbors(node_id: u32, depth: u8) -> Result<Vec<NodeImpact>>`: ดึงโหนดเพื่อนบ้านในระดับลึกพร้อมจัดลำดับตาม [[ALGO--KIMPACT-CALCULATION]]
*   `add_edge(src: u32, tgt: u32, rel: RelType, flags: u8) -> Result<()>`: สร้างความสัมพันธ์พร้อมตรวจสอบ [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
*   `query(cypher: &str) -> Result<ResultSet>`: รองรับการสืบค้นรูปแบบ Cypher สำหรับโครงสร้างที่ซับซ้อน
*   `compact() -> Result<Stats>`: รวบรวมและบีบอัดข้อมูลลงดิสก์แบบ Atomic ตาม [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]]

## 3. Technical Constraints
*   **Memory Safety:** การส่งผ่านข้อมูลผ่าน FFI ต้องไม่มี Memory Leak และรองรับ Thread-safe Access.
*   **Serialization:** ใช้ format ที่มี overhead ต่ำ (เช่น FlatBuffers หรือ Bincode) สำหรับความสัมพันธ์ขนาดใหญ่.
*   **Error Handling:** ต้องส่งคืน Error Codes ที่สอดคล้องกับ Axiomatic Paradox หรือ System Faults.

## 4. Performance Requirements
*   **Context Switch:** Overhead ของ FFI call ต้อง < 10µs.
*   **Throughput:** รองรับ 25,000 Edge Mutations ต่อวินาที.
