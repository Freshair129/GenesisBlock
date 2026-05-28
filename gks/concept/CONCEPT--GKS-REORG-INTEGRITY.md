---
id: CONCEPT--GKS-REORG-INTEGRITY
phase: 1
type: concept
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: GKS Semantic Reorganization — Documentation integrity and graph stability
created_at: 2026-05-28T15:00:00.000+07:00
tags: [concept, gks, reorg, integrity]
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - CONCEPT--TAXONOMY-V2-3
    - ADR--FLAT-ATOM-LAYOUT
linked_symbols: []
attributes:
  domain: general
---

# CONCEPT — GKS Semantic Reorganization Integrity

## Problem

ระบบ GKS ปัจจุบันประสบปัญหาข้อมูลกระจัดกระจาย (Fragmentation):
- **Unstable Paths**: มีการใช้ Relative Links (`../`) ซึ่งจะพังทันทีเมื่อย้ายไฟล์
- **ID Inconsistency**: ไฟล์บางส่วนไม่มี ID หรือ ID ไม่ตรงกับชื่อไฟล์
- **Taxonomy Drift**: ชื่อโฟลเดอร์ไม่เป็นเอกพจน์ (Singular) ตามที่กำหนดใน `atom_registry.yaml`

## Hypothesis

หากเราเปลี่ยนจากการเชื่อมโยงแบบ Path ไปเป็น **ID-linked Semantic Graph** และบังคับใช้โครงสร้างโฟลเดอร์แบบ Singular ระบบจะสามารถรักษาความถูกต้องของข้อมูล (Integrity) และ "เยียวยาตัวเอง" (Self-healing) ได้เมื่อมีการปรับโครงสร้าง

## Scope

- ตรวจสอบและสร้าง ID ให้กับทุก Atom ใน `gks/`
- เปลี่ยน Relative Links ทั้งหมดเป็น Wikilinks `[[ID]]`
- ย้ายไฟล์เข้าโฟลเดอร์เอกพจน์ (Singular Folders)
- อัปเดต `atomic_index.jsonl`

## Out of scope

- การแก้ไขเนื้อหาภายใน Atom (ยกเว้นส่วน Link และ Frontmatter)
- การจัดระเบียบไฟล์โค้ดใน `packages/`

## Verification

- `npm run msp:index` ทำงานสำเร็จ
- `npm run msp:check-links` ไม่พบ Broken Links
- ทุก Atom อยู่ในโฟลเดอร์ที่ถูกต้องตาม Registry

## Source

- User direction for GKS reorganization and cleanup.
