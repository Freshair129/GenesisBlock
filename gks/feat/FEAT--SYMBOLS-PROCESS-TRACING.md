---
id: FEAT--SYMBOLS-PROCESS-TRACING
phase: 2
type: feat
status: superseded
tier: process
source_type: axiomatic
vault_id: default
title: End-to-end process tracing — following execution flows from entry points
  to leaf functions
tags: &a1
  - msp
  - symbol-graph
  - data-flow
  - trace
  - impact-analysis
  - feat
crosslinks: &a2
  implements:
    - ADR--SYMBOL-GRAPH-PERSISTENCE
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--SYMBOL-GRAPH
    - FEAT--SYMBOLS-FRAMEWORK-AWARENESS
  superseded_by:
    - CONCEPT--SYMBOLS-PROCESS-TRACING
    - ADR--SYMBOLS-PROCESS-TRACING
    - ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL
    - PROTO--SYMBOLS-TRACE-INVARIANTS
linked_symbols: &a3
  - file: packages/msp/src/symbols/tracer.ts
  - file: packages/msp/src/mcp/tools/symbol-trace.ts
created_at: 2026-05-12T04:48:00.000+07:00
aliases: &a4
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  id: FEAT--SYMBOLS-PROCESS-TRACING
  phase: 2
  type: feat
  status: superseded
  tier: process
  source_type: axiomatic
  vault_id: default
  title: End-to-end process tracing — following execution flows from entry points
    to leaf functions
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-12T04:48:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Feature spec
  attributes:
    id: FEAT--SYMBOLS-PROCESS-TRACING
    phase: 2
    type: feat
    status: superseded
    tier: process
    source_type: axiomatic
    vault_id: default
    title: End-to-end process tracing — following execution flows from entry points
      to leaf functions
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-12T04:48:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Feature spec
    attributes:
      domain: feat
    domain: feat
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# FEAT — End-to-end process tracing

## User-facing behaviour

เพิ่มความสามารถในการติดตาม "สายใยการทำงาน" (Execution Flows) ตั้งแต่จุดเริ่มต้น (Entry points) ไปจนถึงปลายทาง (Leaf functions/External calls) เพื่อให้เห็นภาพการไหลของข้อมูลในระบบอย่างสมบูรณ์:

1. **Trace Engine (Stage 12):** พัฒนาโมดูลใน `src/symbols/tracer.ts` ที่สามารถเดินตาม `CALLS` edges อย่างเป็นลำดับเพื่อสร้าง "แผนที่เส้นทาง" (Execution Path)
2. **Entry Point Identification:** ค้นหาจุดเริ่มต้นอัตโนมัติ (เช่น API Routes, Tool Handlers, Scheduled Jobs)
3. **Execution Flow Visualization:** สามารถสกัด Subgraph ที่แสดงเฉพาะเส้นทางการทำงานของกระบวนการใดกระบวนการหนึ่ง (เช่น "Flow การสมัครสมาชิก") ออกมาให้ AI วิเคราะห์
4. **Impact Analysis (Deep):** ช่วยให้ AI วิเคราะห์ได้ว่า "หากแก้โค้ดที่ฟังก์ชันนี้ จะส่งผลกระทบต่อกระบวนการธุรกิจใดบ้าง?" (Tracing upwards to entry points)

เพิ่ม MCP Tool ใหม่: `msp_symbol_trace` เพื่อให้ Agent เรียกใช้ในการสืบค้นเส้นทางการไหลของข้อมูล

## Why Process Tracing

เพื่อให้ AI สามารถทำความเข้าใจ "พฤติกรรม" ของระบบ (Runtime behavior simulation) ได้จากโค้ดต้นฉบับ:
- ช่วยลดความผิดพลาดในการแก้ไขโค้ดที่ซับซ้อน (Spaghetti code) โดยการแสดงเส้นทางที่ชัดเจน
- ช่วยในการทำ Onboarding ให้กับ Developer หรือ Agent ใหม่ โดยการแสดงลำดับการทำงานที่เกิดขึ้นจริงในระบบ

## Verification

- **Unit Test:** `test/symbols/tracer.test.ts` ตรวจสอบความถูกต้องของการเดิน Graph จากจุดเริ่มต้นไปยังจุดสิ้นสุดที่กำหนด
- **Impact Test:** ทดสอบการทำ "Reverse Trace" จากฟังก์ชันระดับล่างสุดขึ้นมายัง API Route ว่าได้ผลลัพธ์ครบถ้วนหรือไม่
- **MCP Verification:** เรียก `msp_symbol_trace` บน API Route ตัวอย่าง แล้วตรวจสอบว่าคืนค่าเป็นลำดับขั้นการเรียก (Call Stack-like structure) ที่ถูกต้อง

## Out of scope

- การวิเคราะห์ Dynamic Dispatch หรือ Reflection ที่ซับซ้อนเกินกว่า Static Analysis จะทำได้
- การติดตามสถานะของข้อมูล (Variable state tracking) ในระดับบรรทัด (เน้นที่ระดับ Function/Method call ก่อน)
- การวิเคราะห์ข้าม Services (Distributed tracing) — เฟสนี้เน้นภายใน Monorepo เดียวกันก่อน

## Source

- `[[FEAT--SYMBOLS-FRAMEWORK-AWARENESS]]`
- `packages/msp/src/symbols/communities/leiden.ts` (ใช้โครงสร้าง Graph ที่ผ่านการจัดกลุ่มแล้วมาช่วยในการวิเคราะห์ Flow)
- `[[ADR--SYMBOL-GRAPH-PERSISTENCE]]`
- Concept of "Neural Mapping" for code intelligence

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--SYMBOL-GRAPH]]
- [[CONCEPT--SYMBOLS-PROCESS-TRACING]]
- [[ADR--SYMBOLS-PROCESS-TRACING]]
- [[ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL]]
- [[PROTO--SYMBOLS-TRACE-INVARIANTS]]

