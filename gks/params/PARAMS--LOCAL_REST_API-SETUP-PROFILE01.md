---
id: PARAMS--LOCAL_REST_API-SETUP-PROFILE01
phase: 2
type: params
status: active
vault_id: default
tier: process
source_type: axiomatic
title: Standard Local REST API Configuration Parameters
created_at: 2026-05-29T12:10:00.000+07:00
last_updated: 2026-05-29T12:10:00.000+07:00
cluster: standards
role: Parameter definition
crosslinks:
  references:
    - RUNBOOK--OBSIDIAN-COM_PG-LOCAL_REST_API-SETUP
attributes:
  domain: standards
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PARAMS — Local REST API Setup Profile 01

## Purpose

เอกสารนี้กำหนดค่าพารามิเตอร์มาตรฐานสำหรับปลั๊กอิน **Local REST API** ใน Obsidian ซึ่งใช้เปิด Endpoint เพื่อเชื่อมต่อและรับส่งข้อมูลระหว่างสภาพแวดล้อม Obsidian ของผู้ใช้กับ Agent ภายนอกผ่านโปรโตคอล MCP (Model Context Protocol)

## Values / Thresholds

| Parameter Name | Target Value | Recommended State | Technical Description |
| :--- | :--- | :---: | :--- |
| `port` | `27124` | **🟢 Custom** | พอร์ตสำหรับการสื่อสารแบบ HTTPS (Secure Server) |
| `insecurePort` | `27123` | **🟢 Custom** | พอร์ตสำหรับการสื่อสารแบบ HTTP (Insecure Server) ใช้กับการเชื่อมต่อ Local MCP ที่จำกัดสิทธิ์ในเครื่อง |
| `enableInsecureServer` | `true` | **🟢 ON** | เปิดใช้บริการพอร์ต HTTP เพื่อความง่ายและความเร็วในเครื่องคอมพิวเตอร์โลคัล |
| `enableSecureServer` | `true` | **🟢 ON** | เปิดใช้บริการ HTTPS เพื่อรองรับความปลอดภัยที่สูงขึ้น |
| `apiKey` | `"a50f9d51483273acfefcc46abb2ddd24e6598bfb7ac1d85fcc137377be5ef708"` | **🟢 Fixed** | คีย์สำหรับรับรองสิทธิ์ (Authentication Token) ที่ระบบกำหนดไว้เพื่อการเข้าถึง REST API |

## Update Policy

- **Storage:** การตั้งค่าภายในโปรแกรม Obsidian Client (`.obsidian/plugins/obsidian-local-rest-api/data.json`)
- **Update Frequency:** ปรับปรุงเมื่อต้องการเปลี่ยนพอร์ตเชื่อมต่อหรือรีเซ็ต API Key ด้วยเหตุผลด้านความปลอดภัย
- **Authority:** จะต้องได้รับความเห็นชอบผ่านกระบวนการเสนอและอนุมัติ ADR/PARAMS เสมอก่อนปรับค่า

*End of Parameter Profile*
