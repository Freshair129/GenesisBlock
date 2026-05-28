---
id: RUNBOOK--OBSIDIAN-LINTER-SETUP
phase: 6
type: runbook
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Obsidian Linter Plugin Configuration Standard for GKS Workspace
created_at: 2026-05-29T02:22:57.000+07:00
tags:
  - obsidian
  - linter
  - configuration
  - setup
  - ops
aliases:
  - Obsidian Linter Setup SOP
  - Linter Setup Guide
cluster: ops
role: Operational response guide
crosslinks:
  depends_on:
    - PARAMS--LINTER-SETUP-PROFILE01
  references:
    - CONCEPT--OBSIDIAN-AS-RUNTIME
attributes:
  domain: operations
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Obsidian Linter Setup Guide

เอกสารคู่มือขั้นตอนการตั้งค่าปลั๊กอิน Obsidian Linter ในสภาพแวดล้อมการเขียนเอกสารของ GKS เพื่อให้เป็นไปตามข้อกำหนดความปลอดภัยของระบบโครงสร้างข้อมูล

## Trigger

- เมื่อเตรียมสภาพแวดล้อมการทำงานใหม่ (New Developer Environment Setup)
- เมื่อตรวจพบความเสียหายของโครงสร้าง frontmatter หลังกดเซฟไฟล์ใน Obsidian
- เมื่อมีสคริปต์แจ้งเตือนความผิดพลาดของ YAML Syntax จากระบบ `npm run msp:validate`

## Response steps

### 1. การติดตั้งปลั๊กอิน
1. เปิดโปรแกรม Obsidian
2. ไปที่ **Settings** > **Community plugins** > **Browse**
3. ค้นหาปลั๊กอิน **Linter** (โดยผู้พัฒนา *platers*)
4. กด **Install** และ **Enable** เพื่อเปิดใช้งาน

### 2. การตั้งค่ากำหนดตามโปรไฟล์มาตรฐาน
1. เข้าไปที่หน้าการตั้งค่า **Linter** ใน Obsidian Settings
2. ปรับแต่งค่าในหมวดหมู่ **YAML** ตามพารามิเตอร์ที่ระบุไว้ในเอกสารอ้างอิง [PARAMS--LINTER-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--LINTER-SETUP-PROFILE01.md)
3. ตรวจสอบให้มั่นใจว่าตัวเลือกที่เป็นอันตราย (เช่น Key Sort, Sort Array และ Timestamp) ถูกปิดอย่างถูกต้องเพื่อรักษาความถูกต้องของ Schema

### 3. การตรวจสอบความถูกต้องหลังบันทึก
ทุกครั้งที่มีการจัดฟอร์แมต ให้ทดสอบโดยการรันสคริปต์ตรวจสอบความถูกต้องผ่าน Terminal ที่โฟลเดอร์ Root ของโปรเจกต์:
```bash
npm run msp:validate
```

## Source

- [atom_schema.yaml](file:///c:/Users/freshair/cognitive_system/atom_schema.yaml) — GKS frontmatter standard schema
- [PARAMS--LINTER-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--LINTER-SETUP-PROFILE01.md) — Linter standard parameters profile
