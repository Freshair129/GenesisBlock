---
id: PARAMS--METADATA-SETUP-PROFILE01
title: Recommended Metadata Profile — Metadata Menu Plugin (Obsidian)
cluster: standards
role: Parameter definition
crosslinks:
  references:
    - RUNBOOK--OBSIDIAN-METADATA-SETUP
    - RUNBOOK--OBSIDIAN-LINTER-SETUP
attributes:
  domain: standards
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PARAMS — Recommended Metadata Profile for Metadata Menu Plugin

Profile นี้กำหนดค่ามาตรฐานสำหรับ **Metadata Menu plugin** (v0.8.12 by mdelobelle) ในการจัดการ front‑matter metadata ของ GKS Atoms ใน Obsidian เพื่อให้ระบบ Cross‑Link แสดง property ได้ครบถ้วน

---

## ส่วนที่ 1 — Global Settings (Settings → Metadata Menu)

ตั้งค่าที่นี่จะมีผลกับทุก note ใน vault

| Setting | ค่าที่แนะนำ | รายละเอียด |
|---------|------------|------------|
| **Display field options in context menu** | ✅ ON | เปิดให้ right-click บนโน้ตแล้วเห็น field options ได้ทันที |
| **Automatically save after field change** | ❌ OFF | ปิดไว้เพื่อป้องกัน Obsidian auto-save ที่อาจทำลาย frontmatter structure ของ GKS |
| **Manage all properties** | ❌ OFF | ให้ plugin จัดการเฉพาะ field ที่กำหนดไว้ใน Preset เท่านั้น ไม่ไปยุ่งกับ field อื่น |
| **Add missing fields button** | ✅ ON | เปิดปุ่ม "Add missing fields" เพื่อให้เพิ่ม field ที่หายได้ง่ายผ่าน UI |
| **Metadata button in file explorer** | ✅ ON | แสดงปุ่ม metadata icon ข้าง note เพื่อเข้าถึง form แก้ไขได้รวดเร็ว |
| **fileClass files folder** | `_fileclasses/` | path ของ folder ที่เก็บไฟล์ fileClass (ดูส่วนที่ 3) |

---

## ส่วนที่ 2 — Preset Field Settings (Global Fields)

ไปที่ **Settings → Metadata Menu → Preset Field settings** แล้วกด **"+ Add"** ทีละ field

### `domain`
| ตัวเลือก | ค่า |
|----------|-----|
| **Type** | `Select` |
| **Preset values** | `knowledge`, `policy`, `operations`, `standards`, `security` |
| **Default** | `knowledge` |
| **Description** | หมวดหมู่หลักของ Atom — ใช้ใน Cross‑Link grouping และ UI badge |

> **ทำไมต้อง Select?**  
> บังคับให้เลือกจาก list ที่กำหนด ป้องกันการพิมพ์ผิด เช่น `knwoledge` หรือ `Knowledge` (ตัวพิมพ์ใหญ่) ซึ่งจะทำให้ GKS validator reject

---

### `source_type`
| ตัวเลือก | ค่า |
|----------|-----|
| **Type** | `Select` |
| **Preset values** | `gks`, `msp`, `external` |
| **Default** | `gks` |
| **Description** | แหล่งที่มาของ Atom — ใช้แสดง icon ต่างกันใน UI |

> **ทำไมต้อง Select?**  
> ค่านี้มีผลต่อ rendering logic ใน GKS Cross‑Link renderer ถ้าใส่ค่าที่ไม่อยู่ใน list เช่น `GKS` หรือ `Gks` จะทำให้ icon ไม่แสดง

---

### `created_at`
| ตัวเลือก | ค่า |
|----------|-----|
| **Type** | `DateTime` |
| **Format** | `YYYY-MM-DDTHH:mm:ss.SSS+07:00` |
| **Insert on creation** | ✅ ON |
| **Description** | เวลาที่สร้าง Atom — ต้องมี timezone offset `+07:00` ไม่ใช่ `Z` |

> **ทำไม +07:00 ไม่ใช่ Z?**  
> ระบบ GKS ใช้ ICT (UTC+7) เป็น wall-clock time ตาม rule ใน AGENT.md และ GEMINI.md  
> Validator จะ reject ค่าที่มี `Z` suffix เพราะไม่ตรงกับ format ที่กำหนด

---

### `summary`
| ตัวเลือก | ค่า |
|----------|-----|
| **Type** | `Input` (Text) |
| **Default** | (ว่าง) |
| **Description** | คำอธิบาย 1-2 ประโยค — แสดงใน Cross‑Link tier `SUMMARY` และ hover card |

> **ทำไมสำคัญ?**  
> ถ้าไม่มี `summary` ระบบจะ fallback ไปใช้ `SKELETON` tier แทน ทำให้ผู้ใช้เห็นข้อมูลน้อยลง

---

### `tags`
| ตัวเลือก | ค่า |
|----------|-----|
| **Type** | `Multi` |
| **Preset values** | `onboarding`, `security`, `important`, `deprecated`, `draft` |
| **Description** | Tag list สำหรับกรองและค้นหาใน Dataview/UI |

---

## ส่วนที่ 3 — FileClass Setup (Per‑Type Schema)

ใช้ fileClass เมื่อต้องการให้ Atom แต่ละประเภท (`CONCEPT--*`, `ADR--*`) มี field schema คนละชุด

### 3.1 สร้าง folder
```
cognitive_system/
└── _fileclasses/
    ├── GKSAtom.md        ← ใช้กับ Atom ทั่วไป
    ├── GKSConcept.md     ← ใช้กับ CONCEPT--* เฉพาะ
    └── GKSADR.md         ← ใช้กับ ADR--* เฉพาะ
```

### 3.2 ตั้งค่า path ใน plugin
**Settings → Metadata Menu → fileClass files folder:** `_fileclasses/`

### 3.3 Folder Mapping (อัตโนมัติ ไม่ต้องเพิ่ม frontmatter ทุกไฟล์)
เปิดไฟล์ `GKSAtom.md` → คลิก icon ที่ tab header → **FileClass Settings** → เพิ่ม Folder Mapping:

| Folder | FileClass |
|--------|-----------|
| `gks/concept/` | `GKSAtom` |
| `gks/adr/` | `GKSAtom` |
| `gks/feat/` | `GKSAtom` |
| `gks/blueprint/` | `GKSAtom` |
| `gks/runbook/` | `GKSAtom` |
| `gks/params/` | `GKSAtom` |

---

## ส่วนที่ 4 — Example front‑matter (ครบทุก field)

```yaml
---
id: CONCEPT--EXAMPLE-NOTE
title: ตัวอย่างโน้ต
domain: knowledge
source_type: gks
created_at: 2026-05-29T09:15:00.000+07:00
summary: คำอธิบาย 1-2 ประโยค สำหรับแสดงใน Cross-Link
tags:
  - onboarding
  - security
attributes:
  salient: "จุดเด่นของโน้ต"
  trigger: "เมื่อผู้ใช้เปิด Obsidian ครั้งแรก"
  hook: "link-to:CONCEPT--RELATED"
---
```

---

## ส่วนที่ 5 — Validation Checklist

หลังตั้งค่าแล้วให้ตรวจสอบดังนี้:

- [ ] `domain` มีค่าเป็นหนึ่งใน: `knowledge`, `policy`, `operations`, `standards`, `security`
- [ ] `source_type` มีค่าเป็นหนึ่งใน: `gks`, `msp`, `external`
- [ ] `created_at` มีรูปแบบ `YYYY-MM-DDTHH:mm:ss.SSS+07:00` (ไม่ใช่ `Z`)
- [ ] `summary` อยู่ในบรรทัดเดียว ไม่เกิน 200 ตัวอักษร
- [ ] `tags` เป็น YAML list (ไม่ใช่ inline เช่น `[a, b]`)
- [ ] รัน `npm run msp:validate` ผ่านโดยไม่มี error

---

*End of Parameter Profile*
