---
id: ADR--GKS-SINGULAR-TAXONOMY
tier: genesis
created_at: 2026-05-28T15:05:00.000+07:00
phase: 2
type: adr
status: active
vault_id: GKS-CORE
title: Enforcing Singular Folder Taxonomy and ID-based Linking
tags: [architecture, gks, taxonomy]
domain: documentation
crosslinks:
  references:
    - CONCEPT--GKS-REORG-INTEGRITY
    - ADR--FLAT-ATOM-LAYOUT
  supersedes: []
  resolves: []
linked_symbols: []
---

# ADR — Enforcing Singular Folder Taxonomy

## Context

`atom_registry.yaml` กำหนดให้ใช้ชื่อโฟลเดอร์แบบเอกพจน์ (Singular) สำหรับแต่ละประเภทของ Atom แต่ปัจจุบันในโปรเจกต์มีการใช้พหูพจน์ (เช่น `adrs/`) ซึ่งขัดกับ MSP Validator และทำให้การสแกนข้อมูลของ Agent สับสน

## Decision

เราตัดสินใจ **บังคับใช้กฎ Singular Folder** และเปลี่ยนการเชื่อมโยงทั้งหมดใน GKS ให้เป็น **ID-based Wikilinks** ทันที เพื่อรองรับการย้ายไฟล์โดยไม่กระทบความสัมพันธ์

## Consequences

**Positive**

- ระบบสอดคล้องกับ Master Spec และ Validator
- ย้ายไฟล์ระหว่างโฟลเดอร์ได้โดยลิงก์ไม่พัง
- สื่อสารระหว่าง Agent ได้แม่นยำขึ้นผ่าน ID

**Negative**

- ลิงก์แบบ Relative Path เดิมจะพังทั้งหมด (ต้องทำ Bulk Replacement)

## Alternatives considered

1. **คงโครงสร้างเดิมไว้** — *Rejected.* เพราะขัดกับมาตรฐานใหม่และทำให้การขยายระบบยากขึ้น
2. **ใช้การ Symlink โฟลเดอร์** — *Rejected.* ซับซ้อนเกินไปและไม่แก้ปัญหาที่ต้นเหตุ

## References

- `atom_registry.yaml`
- `FRAMEWORK_MASTER_SPEC.md`
