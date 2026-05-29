---
id: RUNBOOK--OBSIDIAN-COM_PG-LIVESYNC-SETUP
phase: 6
type: runbook
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Obsidian Self-hosted LiveSync Setup Guide
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: ops
role: Operational response guide
crosslinks:
  depends_on:
    - PARAMS--LIVESYNC-SETUP-PROFILE01
attributes:
  domain: operations
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Obsidian Self-hosted LiveSync Setup Guide

เอกสารนี้อธิบายขั้นตอนการติดตั้งฐานข้อมูล CouchDB บนระบบปฏิบัติการ Windows, การเปิดใช้งาน CORS, การสร้างฐานข้อมูล และการตั้งค่าไฟล์คอนฟิกของปลั๊กอิน **Self-hosted LiveSync** ใน Obsidian

## Trigger

- เมื่อเตรียมการติดตั้งระบบซิงโครไนซ์บันทึกข้อมูลส่วนตัว (Self-hosted Sync)
- เมื่อพบความล่าช้าในการส่งข้อมูล หรือเชื่อมต่อ CouchDB ล้มเหลว

---

## Response steps

### 1️⃣ การติดตั้ง CouchDB บน Windows
1. ดาวน์โหลดตัวติดตั้ง `.exe` สำหรับ Windows จากเว็บบอร์ดทางการ: [CouchDB Download](https://couchdb.apache.org/#download)
2. รันตัวติดตั้งและทำตามขั้นตอนในหน้าต่างการตั้งค่า
3. กำหนดรหัสผ่านของผู้ดูแลระบบ (Admin Password) เมื่อถูกถาม และบันทึกรหัสผ่านไว้เพื่อกรอกในคอนฟิก

---

### 2️⃣ การเปิดใช้งาน CORS และสร้างฐานข้อมูลผ่าน REST API
เมื่อติดตั้ง CouchDB เรียบร้อยแล้ว ฐานข้อมูลจะรันที่พอร์ต `5984` ให้รันคำสั่ง PowerShell ต่อไปนี้ใน Terminal เพื่อตรวจสอบการเข้าถึง เปิดใช้งาน CORS และสร้างฐานข้อมูล `gks-sync`

> โปรดแทนที่ `<ADMIN_USER>` และ `<ADMIN_PASSWORD>` ด้วยชื่อผู้ใช้งานและรหัสผ่านจริงของคุณ

#### 1. ตรวจสอบการทำงานของ CouchDB:
```powershell
Invoke-RestMethod -Uri "http://127.0.0.1:5984/" -Method Get
```

#### 2. เปิดใช้งาน CORS (Cross-Origin Resource Sharing) เพื่อให้ Obsidian ติดต่อได้:
```powershell
# เปิดใช้งาน CORS ทั่วไป
Invoke-RestMethod -Uri "http://127.0.0.1:5984/_node/nonode@nohost/_config/httpd/enable_cors" -Method Put -Credential (Get-Credential) -Body '"true"' -ContentType "application/json"

# อนุญาต Origins ทั้งหมด (หรือระบุ origin ของ Obsidian คอนฟิก)
Invoke-RestMethod -Uri "http://127.0.0.1:5984/_node/nonode@nohost/_config/cors/origins" -Method Put -Credential (Get-Credential) -Body '"*"' -ContentType "application/json"

# อนุญาต Credentials และ Headers ที่จำเป็น
Invoke-RestMethod -Uri "http://127.0.0.1:5984/_node/nonode@nohost/_config/cors/credentials" -Method Put -Credential (Get-Credential) -Body '"true"' -ContentType "application/json"

Invoke-RestMethod -Uri "http://127.0.0.1:5984/_node/nonode@nohost/_config/cors/headers" -Method Put -Credential (Get-Credential) -Body '"accept, authorization, content-type, origin, referer"' -ContentType "application/json"

Invoke-RestMethod -Uri "http://127.0.0.1:5984/_node/nonode@nohost/_config/cors/methods" -Method Put -Credential (Get-Credential) -Body '"GET, PUT, POST, HEAD, DELETE"' -ContentType "application/json"
```

#### 3. สร้างฐานข้อมูลสำหรับการซิงก์ข้อมูล:
```powershell
Invoke-RestMethod -Uri "http://127.0.0.1:5984/gks-sync" -Method Put -Credential (Get-Credential)
```

---

### 3️⃣ ตั้งค่าพารามิเตอร์ของปลั๊กอินใน Obsidian
ปิดโปรแกรม Obsidian และเขียนทับการตั้งค่าในไฟล์ `c:\Users\freshair\cognitive_system\gks\.obsidian\plugins\obsidian-livesync\data.json` ด้วยฟิลด์และข้อมูลรหัสผ่านที่ถูกต้อง:

```json
{
  "remoteType": "couchdb",
  "couchDB_URI": "http://localhost:5984",
  "couchDB_USER": "<ADMIN_USER>",
  "couchDB_PASSWORD": "<ADMIN_PASSWORD>",
  "couchDB_DBNAME": "gks-sync",
  "deviceAndVaultName": "windows-pc",
  "liveSync": true,
  "syncOnSave": false,
  "isConfigured": true
}
```

เปิดโปรแกรม Obsidian เพื่อเริ่มการทำงานซิงโครไนซ์

---

## Source

- [PARAMS--LIVESYNC-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--LIVESYNC-SETUP-PROFILE01.md)

*End of Runbook*
