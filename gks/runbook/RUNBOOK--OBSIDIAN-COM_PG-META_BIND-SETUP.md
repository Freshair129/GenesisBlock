---
id: RUNBOOK--OBSIDIAN-COM_PG-META_BIND-SETUP
phase: 6
type: runbook
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Obsidian Meta Bind Plugin Setup Guide
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: ops
role: Operational response guide
crosslinks:
  depends_on:
    - PARAMS--META_BIND-SETUP-PROFILE01
attributes:
  domain: operations
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Obsidian Meta Bind Setup Guide

เอกสารนี้อธิบายขั้นตอนการติดตั้งและตั้งค่าโปรไฟล์ปลั๊กอิน **Meta Bind** เพื่อกำหนดรูปแบบเวลาและโครงสร้างการป้อนข้อมูลให้สอดคล้องกับกฎของ GKS Workspace

## Trigger

- เมื่อติดตั้งสภาพแวดล้อมใหม่สำหรับนักพัฒนาซอฟต์แวร์
- เมื่อพบความไม่สอดคล้องของ DateTime ใน Metadata ฟิลด์ที่สร้างโดย Meta Bind (เช่น ใช้ `Z` แทนที่จะใช้ ICT `+07:00`)
- เมื่อพบปัญหาว่าไฟล์ดัชนีใน `00_index` ถูกแก้ไขโดยไม่ได้ตั้งใจ

---

## Response steps

### 1️⃣ ติดตั้งและเปิดใช้งาน Plugin
1. เปิดโปรแกรม Obsidian
2. ไปที่ **Settings** → **Community plugins** → **Browse**
3. ค้นหา **"Meta Bind"** (โดยผู้พัฒนา *mProjects*)
4. กด **Install** และ **Enable** เพื่อเปิดใช้งาน

---

### 2️⃣ ตั้งค่าพารามิเตอร์แบบ Manual / Script-driven
ตามข้อตกลง ค่าพารามิเตอร์ของ Meta Bind จะต้องได้รับการกำหนดค่าตาม [PARAMS--META_BIND-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--META_BIND-SETUP-PROFILE01.md)

เพื่อหลีกเลี่ยงการตั้งค่าผิดพลาดทางหน้าจอ UI คุณสามารถเขียนทับไฟล์การตั้งค่าทางตรงได้ดังนี้:
1. ปิดโปรแกรม Obsidian
2. แก้ไขไฟล์ `c:\Users\freshair\cognitive_system\gks\.obsidian\plugins\obsidian-meta-bind-plugin\data.json`
3. อัปเดตคีย์ต่อไปนี้:
   ```json
   {
     "preferredDateFormat": "YYYY-MM-DDTHH:mm:ss.SSSZ",
     "excludedFolders": [
       "templates",
       "00_index"
     ]
   }
   ```
4. เปิดโปรแกรม Obsidian ใหม่

---

### 3️⃣ การตรวจสอบความถูกต้อง
ตรวจสอบโดยการรันสคริปต์ตรวจสอบความถูกต้องผ่าน Terminal:
```bash
npm run msp:validate
```
และทดลองสร้าง input bind field ด้วยคำสั่งการป้อนข้อมูลวันเวลาเพื่อให้ระบบยืนยันว่า DateTime ได้รับการเขียนในรูปแบบ UTC Offset `+07:00` อย่างถูกต้อง

---

## Source

- [PARAMS--META_BIND-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--META_BIND-SETUP-PROFILE01.md)
- [atom_schema.yaml](file:///c:/Users/freshair/cognitive_system/atom_schema.yaml)

*End of Runbook*
