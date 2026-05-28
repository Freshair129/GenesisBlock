---
id: PARAMS--LINTER-SETUP-PROFILE01
tier: process
created_at: 2026-05-29T02:22:57.000+07:00
phase: 2
type: params
status: active
vault_id: default
title: Standard Obsidian Linter YAML Configuration Parameters
tags: [config, constants, linter]
domain: operations
crosslinks:
  used_by:
    - RUNBOOK--OBSIDIAN-LINTER-SETUP
---

# PARAMS — Linter Setup Profile 01

## Purpose

เอกสารนี้ทำหน้าที่กำหนดค่าพารามิเตอร์ (Configuration parameters) มาตรฐานสำหรับปลั๊กอิน Obsidian Linter เพื่อให้แน่ใจว่าการจัดรูปแบบไฟล์ Markdown ภายใน GKS workspace จะไม่ทำลายโครงสร้างข้อมูล และเป็นไปตามกฎของระบบ Validator

## Values / Thresholds

| Parameter Name | Target Value | Recommended State | Technical Description |
| :--- | :--- | :---: | :--- |
| `sort-yaml-array-values` | `false` | **🔴 OFF** | ป้องกันลำดับใน `aliases` สลับตำแหน่ง (index 0 ต้องเป็น UPPERCASE) |
| `yaml-key-sort` | `false` | **🔴 OFF** | ป้องกันคีย์เรียงสลับตำแหน่ง และป้องกันการลบบรรทัดว่างใน frontmatter |
| `yaml-timestamp` | `false` | **🔴 OFF** | หลีกเลี่ยงรูปแบบเวลา UTC `Z` (ระบบหลักของ GKS ต้องการ ICT offset `+07:00`) |
| `yaml-title-alias` | `false` | **🔴 OFF** | ห้ามเปลี่ยนค่า aliases อัตโนมัติ เพื่อป้องกันการเขียนทับ type prefix |
| `yaml-title` | `false` | **🔴 OFF** | กำหนดหัวข้อ title เองตามความหมายของเอกสาร |
| `insert-yaml-attributes` | `""` | **🔴 OFF** | ไม่มีความจำเป็นต้องแทรกคีย์ว่างใด ๆ เพิ่มเติมจากเทมเพลตมาตรฐาน |
| `move-tags-to-yaml` | `false` | **🔴 OFF** | ป้องกันการย้ายแท็กจากเนื้อความหลักโดยไม่เจตนา |
| `format-yaml-array` | `true` | **🟢 ON** | จัดระเบียบการย่อหน้าของอาเรย์ให้เป็นมาตรฐานเดียวกัน |
| `default-yaml-array-style` | `single-line` | **🟢 single-line** | ประหยัดพื้นที่การจัดวาง เช่น `tags: [tag1, tag2]` |
| `add-blank-line-after-yaml` | `true` | **🟢 ON** | ใส่บรรทัดว่าง 1 บรรทัดหลัง frontmatter เพื่อให้ตรงตามมาตรฐาน Parser |
| `dedupe-yaml-array-values`| `true` | **🟢 ON** | คัดกรองค่าซ้ำภายในอาเรย์ออกโดยอัตโนมัติ |
| `escape-yaml-special-chars`| `true` | **🟢 ON** | **(สำคัญ)** ครอบอัญประกาศเดี่ยว/คู่ให้ค่าที่มีอักขระพิเศษ ป้องกัน Syntax Error |
| `format-tags-in-yaml` | `true` | **🟢 ON** | ล้างเครื่องหมาย `#` ออกจาก Tags ใน frontmatter อัตโนมัติ |

## Update Policy

- **Storage:** การตั้งค่าภายในโปรแกรม Obsidian Client (ไฟล์ `.obsidian/plugins/obsidian-linter/data.json`)
- **Update Frequency:** มีการปรับปรุงเฉพาะเวลาที่มีการอัปเกรด Schema ของคีย์หน้ากาก GKS (หน้ากากคีย์เวอร์ชันใหม่) เท่านั้น
- **Authority:** จะต้องได้รับความเห็นชอบผ่านกระบวนการเสนอและอนุมัติ ADR/PARAMS เสมอก่อนปรับค่า
