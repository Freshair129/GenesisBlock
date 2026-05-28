---
id: BLUEPRINT--GKS-REORG-RUNBOOK
phase: 3
type: blueprint
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: GKS Semantic Reorganization Execution Runbook
created_at: 2026-05-28T15:10:00.000+07:00
tags: [runbook, implementation, gks]
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - ADR--GKS-SINGULAR-TAXONOMY
    - CONCEPT--GKS-REORG-INTEGRITY
linked_symbols:
  - { file: "scripts/msp/re-indexer.ts" }
attributes:
  domain: general
---

# BLUEPRINT — GKS Semantic Reorganization Runbook

## Architectural Pattern

ใช้ **Recursive Scan & Automated Patching** เพื่อรักษา Semantic Integrity

## Data / Logic

1. **Scan Phase:** ระบุไฟล์ทั้งหมดใน `gks/` และตรวจสอบ ID
2. **Patch Phase:** ใช้ Regex เพื่อหา Relative Links และแทนที่ด้วย ID-based Wikilinks
3. **Move Phase:** ย้ายไฟล์เข้าสู่โฟลเดอร์เอกพจน์
4. **Re-index Phase:** สั่ง Re-index ระบบทั้งหมด

## Geography

- **Source:** `gks/**/*`
- **Output:** `gks/adr/`, `gks/concept/`, `gks/blueprint/`, etc.

## Verification Plan

- **Automated:**
  - `npm run msp:index`
  - `npm run msp:check-links`
  - `npm run msp:validate`
- **Manual:**
  - สุ่มตรวจสอบ 5-10 ไฟล์ใน Obsidian เพื่อยืนยันว่า Graph แสดงผลถูกต้อง

## Source

- Ref: [[ADR--GKS-SINGULAR-TAXONOMY]]
