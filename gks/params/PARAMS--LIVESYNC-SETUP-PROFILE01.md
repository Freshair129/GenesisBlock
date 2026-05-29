---
id: PARAMS--LIVESYNC-SETUP-PROFILE01
phase: 2
type: params
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Recommended Self-hosted LiveSync Configuration Parameters
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: standards
role: Parameter definition
crosslinks:
  references:
    - RUNBOOK--OBSIDIAN-COM_PG-LIVESYNC-SETUP
attributes:
  domain: standards
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PARAMS — Self-hosted LiveSync Setup Profile 01

## Purpose

เอกสารนี้ทำหน้าที่กำหนดค่าพารามิเตอร์มาตรฐานสำหรับปลั๊กอิน **Self-hosted LiveSync** ในการเชื่อมต่อฐานข้อมูล CouchDB โลคัลในระบบ Windows เพื่อการทำสำเนา (Replication) และซิงก์ข้อมูลแบบเรียลไทม์ระหว่างอุปกรณ์

## Values / Thresholds

| Parameter Name | Target Value | Recommended State | Technical Description |
| :--- | :--- | :---: | :--- |
| `remoteType` | `"couchdb"` | **🟢 Fixed** | รูปแบบการเชื่อมต่อระยะไกล (ใช้ฐานข้อมูล CouchDB) |
| `couchDB_URI` | `"http://localhost:5984"` | **🟢 Custom** | ที่อยู่ URI ของฐานข้อมูล CouchDB ในเครื่องโลคัล (หรือ Tailscale IP) |
| `couchDB_DBNAME` | `"gks-sync"` | **🟢 Custom** | ชื่อฐานข้อมูลใน CouchDB ที่จัดสรรไว้สำหรับซิงก์ |
| `liveSync` | `true` | **🟢 ON** | เปิดใช้งานการซิงโครไนซ์แบบเรียลไทม์ (Live Sync) |
| `syncOnSave` | `false` | **🔴 OFF** | ไม่จำเป็นต้องสั่งซิงก์ซ้ำตอนบันทึกเนื่องจาก `liveSync` ทำงานตลอดเวลา |
| `isConfigured` | `true` | **🟢 Fixed** | ยืนยันสถานะการตั้งค่าเสร็จสมบูรณ์ของปลั๊กอิน |

## Update Policy

- **Storage:** การตั้งค่าภายในโปรแกรม Obsidian Client (`.obsidian/plugins/obsidian-livesync/data.json`)
- **Update Frequency:** มีการปรับปรุงเมื่อมีการย้าย Server ฐานข้อมูลหรือเปลี่ยนรหัสผ่านผู้ใช้งาน
- **Authority:** จะต้องได้รับความเห็นชอบผ่านกระบวนการเสนอและอนุมัติ ADR/PARAMS เสมอก่อนปรับค่า

*End of Parameter Profile*
