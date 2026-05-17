---
id: FEAT--SYMBOLS-MULTI-LANG
phase: 2
type: feat
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Multi-language symbol support — adding Python and legacy COBOL parsers to
  MSP graph
tags:
  - msp
  - symbol-graph
  - multi-lang
  - python
  - cobol
  - feat
crosslinks:
  implements:
    - ADR--SYMBOL-GRAPH-PERSISTENCE
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--SYMBOL-GRAPH
    - FEAT--MSP-SYMBOL-MCP
linked_symbols:
  - file: packages/msp/src/symbols/parser/python.ts
  - file: packages/msp/src/symbols/parser/cobol.ts
  - file: packages/msp/src/symbols/api.ts
created_at: 2026-05-12T04:48:00.000+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  domain: feat
---

# FEAT — Multi-language symbol support

## User-facing behaviour

ขยายขีดความสามารถของ `msp:graph build` ให้รองรับการสแกนโค้ดนอกเหนือจาก TypeScript โดยเพิ่ม Parser สำหรับ 2 ภาษาหลักในระยะแรก:

1. **Python Support:** ใช้ `tree-sitter-python` เพื่อสกัดสัญลักษณ์ระดับ Class, Function/Def, และ Async def พร้อมทั้งจับความสัมพันธ์ `CALLS` และ `IMPORTS` ข้ามไฟล์
2. **COBOL Support (Legacy):** ใช้ Regex-based parser เบื้องต้นสำหรับการสกัดสัญลักษณ์พื้นฐาน (PROGRAM-ID, SECTION, DIVISION, CALL) เพื่อให้ระบบสามารถสร้างภาพรวมความสัมพันธ์ข้ามยุคสมัย (Cross-generational mapping) ได้

ระบบจะเลือก Parser อัตโนมัติโดยดูจากนามสกุลไฟล์ (`.py`, `.cbl`, `.cob`, `.ccp`) และบันทึกลงใน Symbol Graph เดียวกัน ทำให้ Agent สามารถใช้ `msp_symbol_lookup` ค้นหาโค้ดได้ทุกภาษาในโปรเจกต์

## Why these 2 languages

- **Python:** เป็นภาษาหลักสำหรับ AI/ML และสคริปต์ในฝั่ง Data/Logic ของหลายโปรเจกต์ การรองรับ Python ทำให้ภาพรวมของระบบประสาท (Neural map) ของซอฟต์แวร์สมบูรณ์ขึ้น
- **COBOL:** เพื่อรองรับการวิเคราะห์ระบบเก่า (Legacy) ซึ่งเป็นความต้องการเชิงกลยุทธ์ในการทำ System Modernization และการสืบค้นความสัมพันธ์เชิงลึกที่ AI เข้ามาช่วยได้มาก

## Verification

- **Unit Tests:** `test/symbols/parser-python.test.ts` และ `test/symbols/parser-cobol.test.ts` ตรวจสอบการสกัดสัญลักษณ์จากโค้ดตัวอย่าง (fixtures)
- **Integration Test:** สแกนโปรเจกต์ที่มีหลายภาษา (polyglot fixture) แล้วตรวจสอบว่า `msp_symbol_search` คืนผลลัพธ์ที่ถูกต้องจากทั้งไฟล์ TS, Python และ COBOL
- **Graph Integrity:** ตรวจสอบว่า `calls` edge สามารถเชื่อมโยงระหว่างภาษาได้ในกรณีที่มีการเรียกข้ามระบบ (เช่น Python เรียก COBOL ผ่าน wrapper)

## Out of scope

- Tree-sitter สำหรับ COBOL (เลื่อนไปเฟสถัดไปเพื่อความรวดเร็วในการ Implement เฟสแรกด้วย Regex)
- การวิเคราะห์ Data Types เชิงลึกใน COBOL (เน้นที่การไหลของ Control Flow และการเรียกใช้งานก่อน)
- ภาษาอื่นๆ (เช่น Java, C++, Go) จะถูกเพิ่มใน FEAT แยกต่างหากในอนาคต

## Source

- `[[CONCEPT--PARSER-CHOICE]]`
- `packages/msp/src/symbols/parser/typescript.ts` (ใช้เป็นแบบอย่างในการทำ Interface)
- `tree-sitter-python` documentation

## Connections
- [[ADR--SYMBOL-GRAPH-PERSISTENCE]]
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--SYMBOL-GRAPH]]
- [[FEAT--MSP-SYMBOL-MCP]]

