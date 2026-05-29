---
id: PARAMS--META_BIND-SETUP-PROFILE01
phase: 2
type: params
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Recommended Meta Bind Plugin Configuration Parameters
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: standards
role: Parameter definition
crosslinks:
  references:
    - RUNBOOK--OBSIDIAN-COM_PG-META_BIND-SETUP
attributes:
  domain: standards
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PARAMS — Meta Bind Setup Profile 01

## Purpose

เอกสารนี้ทำหน้าที่กำหนดค่าพารามิเตอร์มาตรฐานสำหรับปลั๊กอิน **Meta Bind** ใน Obsidian เพื่อให้ระบบสร้าง Date/Time และสเกลโฟลเดอร์สำหรับเอกสาร GKS เป็นไปตามรูปแบบ ICT Timezone offset (`+07:00`) และหลีกเลี่ยงการสแกนหรือแก้ไขไฟล์ดัชนีระบบโดยไม่ตั้งใจ

## Values / Thresholds

| Parameter Name | Target Value | Recommended State | Technical Description |
| :--- | :--- | :---: | :--- |
| `preferredDateFormat` | `"YYYY-MM-DDTHH:mm:ss.SSSZ"` | **🟢 Custom** | จัดรูปแบบ DateTime ให้มี offset timezone (เช่น `+07:00` แทน `Z`) ตรงตามที่ GKS validator ต้องการ |
| `excludedFolders` | `["templates", "00_index"]` | **🟢 Custom** | ยกเว้นโฟลเดอร์ `templates` และ `00_index` จากการประมวลผลและการแก้ไขฟิลด์ |
| `enableJs` | `true` | **🟢 ON** | เปิดใช้งาน JavaScript ใน input fields |
| `ignoreCodeBlockRestrictions` | `true` | **🟢 ON** | อนุญาตการเรนเดอร์ใน code blocks ทั้งหมด |

## Update Policy

- **Storage:** การตั้งค่าภายในโปรแกรม Obsidian Client (`.obsidian/plugins/obsidian-meta-bind-plugin/data.json`)
- **Update Frequency:** มีการปรับปรุงเฉพาะเวลาที่มีการอัปเกรด GKS Date/Time Standards เท่านั้น
- **Authority:** จะต้องได้รับความเห็นชอบผ่านกระบวนการเสนอและอนุมัติ ADR/PARAMS เสมอก่อนปรับค่า

*End of Parameter Profile*
