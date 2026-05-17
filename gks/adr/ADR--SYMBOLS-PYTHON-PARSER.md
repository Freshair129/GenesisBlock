---
id: ADR--SYMBOLS-PYTHON-PARSER
phase: 2
type: adr
status: active
tier: genesis
source_type: axiomatic
vault_id: default
title: ADR — Using Tree-sitter for Python symbol extraction
tags:
  - msp
  - symbol-graph
  - python
  - tree-sitter
  - adr
crosslinks:
  implements:
    - FEAT--SYMBOLS-MULTI-LANG
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--PARSER-CHOICE
created_at: 2026-05-12T05:00:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Using Tree-sitter for Python symbol extraction

## Context

ในการขยายระบบ Symbol Graph ของ MSP ให้รองรับภาษา Python (ตาม `[[FEAT--SYMBOLS-MULTI-LANG]]`) เราจำเป็นต้องเลือกวิธีสแกนรหัสต้นฉบับเพื่อสกัดสัญลักษณ์ (Classes, Functions, Methods, Imports) และความสัมพันธ์ (Calls)

ตัวเลือกที่พิจารณา:
1. **Python `ast` module (via spawned process):** ใช้ตัวสแกนมาตรฐานของ Python เอง
2. **Regex-based:** ใช้ regular expressions สกัดข้อมูลพื้นฐาน
3. **Tree-sitter (Node.js bindings):** ใช้ `tree-sitter-python` ภายใน Node.js โดยตรง

## Decision

เลือกใช้ **Tree-sitter (`tree-sitter-python`)** สำหรับการทำ Python Symbolic Parse

## Rationale

1. **Native Performance:** Tree-sitter รันเป็น native code (C/C++) ภายใน Node.js ทำให้การสแกนไฟล์จำนวนมากรวดเร็วกว่าการ spawn กระบวนการ Python ภายนอกทีละไฟล์
2. **Grammar Accuracy:** Tree-sitter มี grammar ที่แม่นยำและรองรับฟีเจอร์ใหม่ๆ ของ Python 3.x ได้ดีกว่าการใช้ Regex
3. **Uniform AST API:** การใช้ Tree-sitter ทำให้โค้ดในฝั่ง MSP มีรูปแบบการจัดการ AST ที่สอดคล้องกัน (แม้ว่า TypeScript จะใช้ Compiler API แต่การเพิ่มภาษาอื่นๆ ในอนาคตจะใช้ Tree-sitter เป็นมาตรฐาน)
4. **Incremental Parsing:** รองรับการสแกนเฉพาะส่วนที่เปลี่ยนได้ในอนาคต

## Implementation Notes

- ต้องติดตั้ง `tree-sitter` และ `tree-sitter-python` เป็น dependencies ใน `packages/msp`
- สร้าง `src/symbols/parser/python.ts` ที่ implement อินเทอร์เฟซ `SymbolParser`
- Map สัญลักษณ์:
    - `class_definition` → `class`
    - `function_definition` → `function` หรือ `method` (ถ้าอยู่ใน class)
    - `import_from_statement`, `import_statement` → `imports` edges

## Consequences

- **Positive:** ประสิทธิภาพสูง, โค้ด Parser มีโครงสร้างชัดเจน, รองรับความซับซ้อนของภาษาได้ดี
- **Negative:** เพิ่ม Native dependency (`node-gyp` หรือ prebuilt binaries) ซึ่งอาจมีปัญหาในบางสภาพแวดล้อม แต่ยอมรับได้เนื่องจาก MSP รันใน Node.js 20+ เป็นหลัก

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--PARSER-CHOICE]]

