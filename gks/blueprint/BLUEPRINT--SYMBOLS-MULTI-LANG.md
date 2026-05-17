---
id: BLUEPRINT--SYMBOLS-MULTI-LANG
phase: 3
type: blueprint
scale_level: L2
status: active
tier: process
source_type: axiomatic
vault_id: default
title: BLUEPRINT — Multi-language symbol graph expansion (Python + COBOL)
tags:
  - msp
  - symbol-graph
  - multi-lang
  - blueprint
  - python
  - cobol
crosslinks:
  implements:
    - FEAT--SYMBOLS-MULTI-LANG
  references:
    - BLUEPRINT--SYMBOL-GRAPH-CORE
    - ADR--SYMBOLS-PYTHON-PARSER
    - ADR--SYMBOLS-COBOL-STRATEGY
linked_symbols:
  - file: packages/msp/src/symbols/parser/index.ts
  - file: packages/msp/src/symbols/parser/python.ts
  - file: packages/msp/src/symbols/parser/cobol.ts
created_at: 2026-05-12T05:00:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — Multi-language symbol graph expansion

## Scope

พิมพ์เขียวนี้นำเสนอรายละเอียดทางเทคนิคในการขยายระบบ `src/symbols/` ให้รองรับหลายภาษา (Multi-language) โดยเน้นที่การเพิ่ม Python และ COBOL Parser พร้อมทั้งปรับปรุงระบบ Parser Registry ให้สามารถเลือก Parser ได้ตามชนิดของไฟล์

## Architectural pattern: Parser Registry

เราจะปรับปรุงโครงสร้างของ `src/symbols/parser/` ให้เป็นแบบ Registry:

```text
src/symbols/
  parser/
    index.ts        — Parser registry + dispatcher
    typescript.ts   — (Existing) TS Compiler API implementation
    python.ts       — (New) Tree-sitter implementation
    cobol.ts        — (New) Regex implementation
```

ตัวส่ง (Dispatcher) ใน `index.ts` จะทำหน้าที่เลือก Parser:

```typescript
function getParserForFile(filePath: string): SymbolParser {
  const ext = path.extname(filePath).toLowerCase();
  if (['.ts', '.tsx', '.js', '.jsx'].includes(ext)) return TsParser;
  if (['.py'].includes(ext)) return PythonParser;
  if (['.cbl', '.cob', '.ccp'].includes(ext)) return CobolParser;
  return NullParser;
}
```

## Python Parser (Tree-sitter) Implementation

ใช้ `tree-sitter-python` ในการสกัดสัญลักษณ์:
- **Module:** 1 โหนดต่อ 1 ไฟล์ (root node)
- **Class:** `class_definition` → `kind: class`
- **Function:** `function_definition` (ถ้า parent ไม่ใช่ class) → `kind: function`
- **Method:** `function_definition` (ถ้า parent เป็น class) → `kind: method`
- **Imports:** `import_statement`, `import_from_statement` → `imports` edges
- **Calls:** `call` expression → `calls` edges (unresolved ในเฟสแรก)

## COBOL Parser (Regex) Implementation

ใช้การสแกนบรรทัดต่อบรรทัด (Line-by-line scanning):
- **Module:** `PROGRAM-ID. (.*).`
- **Function:** `(.*) SECTION.` หรือ `(.*) DIVISION.`
- **Edges:**
    - `CALL "(.*)"` → `calls` edge (target เป็น module id)
    - `PERFORM (.*)` → `calls` edge (target เป็น local function id)

## Geography

| Module | File |
|---|---|
| Registry | `packages/msp/src/symbols/parser/index.ts` |
| Python Parser | `packages/msp/src/symbols/parser/python.ts` |
| COBOL Parser | `packages/msp/src/symbols/parser/cobol.ts` |
| Type Updates | `packages/msp/src/symbols/types.ts` |
| CLI Update | `packages/msp/src/symbols/cli.ts` |

## Verification plan

### Automated Tests
- **Python:** Fixture `test/fixtures/symbols/sample.py` ต้องสกัดได้ ≥ 5 symbols และ ≥ 2 edges
- **COBOL:** Fixture `test/fixtures/symbols/sample.cbl` ต้องสกัดได้ PROGRAM-ID และ CALL อย่างถูกต้อง
- **Registry:** ทดสอบว่า `getParserForFile` คืนค่า parser ที่ถูกต้องตามนามสกุลไฟล์

### Manual Verification
- รัน `npm run msp:graph build` บนโฟลเดอร์ที่มีไฟล์หลายภาษา
- ตรวจสอบ `symbols.jsonl` ว่ามีสัญลักษณ์จากทั้ง Python และ COBOL ครบถ้วน

## Connections
- [[FEAT--SYMBOLS-MULTI-LANG]]
- [[BLUEPRINT--SYMBOL-GRAPH-CORE]]
- [[ADR--SYMBOLS-PYTHON-PARSER]]
- [[ADR--SYMBOLS-COBOL-STRATEGY]]

