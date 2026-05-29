---
id: RUNBOOK--OBSIDIAN-COM_PG-LOCAL_REST_API-SETUP
phase: 6
type: runbook
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Obsidian Local REST API Plugin Setup Guide
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: ops
role: Operational response guide
crosslinks:
  depends_on:
    - PARAMS--LOCAL_REST_API-SETUP-PROFILE01
attributes:
  domain: operations
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Obsidian Local REST API Setup Guide

เอกสารนี้อธิบายขั้นตอนการติดตั้งและตั้งค่าโปรไฟล์ปลั๊กอิน **Local REST API** ใน Obsidian เพื่อเปิดพอร์ตในการติดต่อสื่อสารกับ Agent ต่าง ๆ ผ่าน Model Context Protocol (MCP)

## Trigger

- เมื่อเตรียมการติดตั้งสภาพแวดล้อมใหม่
- เมื่อ Agent ไม่สามารถอ่านหรือแก้ไขโน้ตใน Obsidian ได้เนื่องจากปัญหาพอร์ตปิด หรือ API Key ไม่ตรงกัน

---

## Response steps

### 1️⃣ ติดตั้งและเปิดใช้งาน Plugin
1. เปิดโปรแกรม Obsidian
2. ไปที่ **Settings** → **Community plugins** → **Browse**
3. ค้นหา **"Local REST API"** (โดยผู้พัฒนา *Vinzent*)
4. กด **Install** และ **Enable** เพื่อเปิดใช้งาน

---

### 2️⃣ ตั้งค่าพารามิเตอร์ของปลั๊กอิน
ตามข้อตกลง ค่าพารามิเตอร์ของ Local REST API จะต้องได้รับการกำหนดค่าตาม [PARAMS--LOCAL_REST_API-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--LOCAL_REST_API-SETUP-PROFILE01.md)

ไฟล์การตั้งค่าจะอยู่ที่:
`c:\Users\freshair\cognitive_system\gks\.obsidian\plugins\obsidian-local-rest-api\data.json`

หากต้องการเขียนทับการตั้งค่าด้วยมือ:
```json
{
  "port": 27124,
  "insecurePort": 27123,
  "enableInsecureServer": true,
  "apiKey": "a50f9d51483273acfefcc46abb2ddd24e6598bfb7ac1d85fcc137377be5ef708",
  "enableSecureServer": true
}
```

*หมายเหตุ: ในการเปิดใช้งานครั้งแรก ปลั๊กอินจะสร้าง Certificate และ Private Key ในฟิลด์ `crypto` อัตโนมัติ ห้ามลบข้อมูลเหล่านั้นหากเปิดใช้งาน HTTPS*

---

### 3️⃣ การตั้งค่าในระดับ MCP Client
การตั้งค่าเชื่อมโยงฝั่ง Agent (เช่น Claude Desktop หรือ Antigravity) จะใช้พารามิเตอร์เชื่อมต่อดังนี้:

- **Method / Transport:** SSE (Server-Sent Events) หรือ HTTP REST
- **Base URL:** `http://localhost:27123` (สำหรับการเชื่อมต่อแบบ HTTP ไม่เข้ารหัสในเครื่อง) หรือ `https://localhost:27124` (HTTPS)
- **Headers / Authentication:**
  - `Authorization: Bearer a50f9d51483273acfefcc46abb2ddd24e6598bfb7ac1d85fcc137377be5ef708`

หากเชื่อมต่อสำเร็จ REST API จะส่งคืนรหัสสถานะ `200 OK` ที่พอร์ต `27123` / `27124`

---

## Source

- [PARAMS--LOCAL_REST_API-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--LOCAL_REST_API-SETUP-PROFILE01.md)

*End of Runbook*
