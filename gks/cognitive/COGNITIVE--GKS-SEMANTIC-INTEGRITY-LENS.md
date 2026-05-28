---
id: COGNITIVE--GKS-SEMANTIC-INTEGRITY-LENS
phase: 2
type: cognitive
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: GKS Semantic Integrity Lens — Decoupling Identity from Path
created_at: 2026-05-28T16:00:00.000+07:00
tags: [cognitive, mental-model, integrity]
aliases:
  - COGNITIVE
  - implementation_flow
  - Mental model / Interpretive lens
cluster: implementation_flow
role: Mental model / Interpretive lens
crosslinks:
  references:
    - CONCEPT--GKS-REORG-INTEGRITY
linked_symbols: []
attributes:
  domain: logic
---

# COGNITIVE — GKS Semantic Integrity Lens

## Intent

ใช้มุมมองที่ว่า "ตัวตนของข้อมูล (Identity) ต้องอยู่เหนือตำแหน่งจัดเก็บ (Location)" เพื่อสร้างระบบความรู้ที่ไม่พังเมื่อมีการขยับขยาย

## Mental Model

1. **ID as Primary Key:** ทรีต Atom ID เป็นค่าคงที่ตลอดกาล
2. **Path as Foreign Key:** ทรีตตำแหน่งไฟล์เป็นเพียงสถานะชั่วคราวที่เปลี่ยนแปลงได้
3. **Link as Semantic Edge:** การเชื่อมโยงคือความสัมพันธ์เชิงตรรกะ ไม่ใช่การชี้ไปยังไฟล์
