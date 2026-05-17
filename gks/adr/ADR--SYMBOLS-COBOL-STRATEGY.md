---
id: ADR--SYMBOLS-COBOL-STRATEGY
phase: 2
type: adr
status: active
tier: genesis
source_type: axiomatic
vault_id: default
title: ADR — Regex-based strategy for legacy COBOL parsing
tags:
  - msp
  - symbol-graph
  - cobol
  - legacy
  - adr
crosslinks: {"implements":["FEAT--SYMBOLS-MULTI-LANG"],"references":["FRAMEWORK--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH"]}
created_at: 2026-05-12T05:00:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — Regex-based strategy for legacy COBOL parsing

## Context

ความต้องการรองรับภาษา COBOL (ตาม `[[FEAT--SYMBOLS-MULTI-LANG]]`) มีวัตถุประสงค์หลักเพื่อการสืบค้นความสัมพันธ์เชิงสถาปัตยกรรม (High-level call mapping) ในระบบ Legacy

ตัวเลือกที่พิจารณา:
1. **Tree-sitter-cobol:** ใช้ grammar ที่สมบูรณ์
2. **Regex-based parser:** ใช้การสแกนบรรทัดต่อบรรทัดเพื่อหาคำสำคัญ (Keywords)

## Decision

เลือกใช้ **Regex-based strategy** ในระยะแรก (Phase 1) สำหรับ COBOL Parser

## Rationale

1. **Lower Complexity:** COBOL เป็นภาษาที่มีความซับซ้อนสูงและมีหลายมาตรฐาน (Dialects) การใช้ Tree-sitter อาจต้องการการตั้งค่าและการปรับแต่งที่สูงเกินความจำเป็นสำหรับความต้องการขั้นต้น
2. **Focus on Entry Points:** เป้าหมายหลักคือการระบุ `PROGRAM-ID`, `SECTION`, `DIVISION` และการ `CALL` ข้ามโปรแกรม ซึ่งสามารถทำได้ค่อนข้างแม่นยำด้วยการใช้ Regular Expressions
3. **Speed of Implementation:** สามารถพัฒนาได้รวดเร็วและไม่มีภาระเรื่อง native bindings เพิ่มเติมในระยะที่ยังไม่ต้องการ AST เชิงลึก
4. **Maintenance:** โค้ด Parser จะอ่านง่ายและปรับแต่งได้ตามหน้างานจริงของระบบ Legacy ที่พบ

## Implementation Notes

- สร้าง `src/symbols/parser/cobol.ts`
- สแกนหา:
    - `PROGRAM-ID. <NAME>.` → `module` symbol
    - `<NAME> SECTION.` → `function` symbol
    - `CALL "<NAME>"` หรือ `PERFORM <NAME>` → `calls` edges
- ตรวจสอบความถูกต้องของสัญลักษณ์โดยมองข้าม Area A/B ของ COBOL ในเบื้องต้น

## Consequences

- **Positive:** เริ่มต้นใช้งานได้ทันที, โค้ดเบาและไม่มี dependencies เพิ่ม
- **Negative:** ไม่สามารถวิเคราะห์ Data Flow เชิงลึกหรือโครงสร้างข้อมูลที่ซับซ้อนได้ หากในอนาคตต้องการรายละเอียดมากขึ้น จะพิจารณาเปลี่ยนไปใช้ Tree-sitter ในเฟสถัดไป

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--SYMBOL-GRAPH]]

