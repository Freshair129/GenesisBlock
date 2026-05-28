---
id: RUNBOOK--OBSIDIAN-METADATA-SETUP
title: Obsidian Metadata Menu Setup Guide
cluster: ops
role: Operational response guide
crosslinks:
  depends_on:
    - PARAMS--METADATA-SETUP-PROFILE01
  references:
    - RUNBOOK--OBSIDIAN-LINTER-SETUP
attributes:
  domain: operations
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Obsidian Metadata Menu Setup Guide

เอกสารนี้อธิบายขั้นตอนการตั้งค่า **Metadata Menu plugin** (v0.8.12, by mdelobelle) ใน Obsidian เพื่อให้ระบบ **Cross‑Link** ของ GKS แสดง property ต่าง ๆ (`domain`, `source_type`, `created_at`, `summary` ฯลฯ) ได้อย่างถูกต้องและครบถ้วน

## Trigger
- พบว่า Cross‑Link ใน UI ไม่แสดง property หรือแสดงค่าเป็น `null`/`undefined`
- สร้าง note ใหม่ที่ต้องการ metadata ครบชุด
- ตั้งค่าสภาพแวดล้อม developer ใหม่
- ระบบ `npm run msp:validate` รายงานข้อผิดพลาดเกี่ยวกับ front‑matter schema

---

## Response steps

### 1️⃣ ติดตั้งและเปิดใช้งาน Plugin
1. เปิดโปรแกรม Obsidian
2. ไปที่ **Settings** → **Community plugins** → **Browse**
3. ค้นหา **"Metadata Menu"** (โดยผู้พัฒนา *mdelobelle*)
4. กด **Install** → **Enable**

> ตรวจสอบเวอร์ชัน: ควรเป็น **v0.8.12** ขึ้นไป

---

### 2️⃣ ตั้งค่า Global Settings

ไปที่ **Settings** → **Metadata Menu** แล้วตั้งค่าดังนี้:

| Setting | ค่าที่แนะนำ | เหตุผล |
|---------|------------|--------|
| **Display field options in context menu** | ✅ ON | right-click บน note แล้วเห็น field options ทันที |
| **Automatically save after field change** | ❌ OFF | ป้องกัน auto-save ทำลาย YAML frontmatter ของ GKS |
| **Manage all properties** | ❌ OFF | จัดการเฉพาะ field ที่กำหนดใน Preset เท่านั้น |
| **Add missing fields button** | ✅ ON | เพิ่ม field ที่หายผ่าน UI ได้ง่าย |
| **Metadata button in file explorer** | ✅ ON | แสดง icon ข้าง note เพื่อเปิด form แก้ไข |
| **fileClass files folder** | `_fileclasses/` | path สำหรับ fileClass schema (ดูขั้นตอนที่ 4) |

---

### 3️⃣ ตั้งค่า Preset Field Settings (Global — ใช้กับทุก note)

ไปที่ **Settings** → **Metadata Menu** → **Preset Field settings** แล้วกด **"+ Add"** ทีละ field:

#### `domain`
- **Type:** `Select`
- **Preset values:** `knowledge`, `policy`, `operations`, `standards`, `security`
- **ทำไม Select?** ป้องกันพิมพ์ผิด เช่น `Knowledge` (ตัวพิมพ์ใหญ่) ซึ่งจะทำให้ validator reject

#### `source_type`
- **Type:** `Select`
- **Preset values:** `gks`, `msp`, `external`
- **ทำไม Select?** ค่านี้มีผลต่อ icon rendering ใน Cross‑Link UI โดยตรง

#### `created_at`
- **Type:** `DateTime`
- **Format:** `YYYY-MM-DDTHH:mm:ss.SSS+07:00`
- **Insert on creation:** ✅ ON
- **ทำไม +07:00?** GKS ใช้ ICT timezone — ถ้าใส่ `Z` จะถูก validator reject

#### `summary`
- **Type:** `Input` (Text)
- **ทำไมสำคัญ?** ถ้าไม่มี summary ระบบ fallback ไปใช้ tier `SKELETON` ทำให้ Cross‑Link แสดงข้อมูลน้อยลง

#### `tags`
- **Type:** `Multi`
- **Preset values:** `onboarding`, `security`, `important`, `deprecated`, `draft`

---

### 4️⃣ ตั้งค่า FileClass (Per‑Folder Schema อัตโนมัติ)

วิธีนี้ทำให้ทุก note ใน folder เช่น `gks/concept/` ได้รับ schema โดยอัตโนมัติ โดยไม่ต้องเพิ่ม `fileClass:` ใน frontmatter ทุกไฟล์

**ขั้นตอน:**

1. สร้าง folder `_fileclasses/` ใน vault root
2. ตั้งค่า path ใน plugin: **Settings** → **Metadata Menu** → **fileClass files folder** → พิมพ์ `_fileclasses/`
3. สร้างไฟล์ `_fileclasses/GKSAtom.md`
4. เปิดไฟล์ `GKSAtom.md` → คลิก icon ที่ tab header → เปิด **FileClass View**
5. ไปที่ **FileClass Settings** → เพิ่ม **Folder Mapping**:

| Folder | FileClass |
|--------|-----------|
| `gks/concept/` | `GKSAtom` |
| `gks/adr/` | `GKSAtom` |
| `gks/feat/` | `GKSAtom` |
| `gks/blueprint/` | `GKSAtom` |
| `gks/runbook/` | `GKSAtom` |
| `gks/params/` | `GKSAtom` |

6. ไปที่ **FileClass Fields** → กด **Add field** แล้วเพิ่มตาม Preset Field Settings ในขั้นตอนที่ 3

> **Note:** ถ้ากำหนด field ทั้งใน Global Preset และ fileClass — **fileClass มี priority สูงกว่าเสมอ**

---

### 5️⃣ ตรวจสอบ front‑matter ตัวอย่าง

หลังตั้งค่าแล้ว note ของคุณควรมี front‑matter ครบชุดดังนี้:

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
  trigger: "เมื่อเกิดเหตุการณ์ X"
  hook: "link-to:CONCEPT--RELATED"
---
```

---

### 6️⃣ รัน Validation
```bash
npm run msp:validate
```
- ✅ ไม่มี error → Metadata ถูกต้อง พร้อมใช้งานใน Cross‑Link
- ❌ มี error → ตรวจสอบ:
  - `domain` ต้องเป็นค่าใน preset list (lowercase)
  - `created_at` ต้องมี `+07:00` (ไม่ใช่ `Z`)
  - `summary` ต้องอยู่ในบรรทัดเดียว
  - `tags` ต้องเป็น YAML list ไม่ใช่ inline array

---

### 7️⃣ ตรวจสอบผลใน UI
```bash
npm run dev --workspace=packages/ui
```
เปิด **Cross‑Link Viewer** → Hover บน note → ควรเห็น:
- Domain badge
- Source‑type icon  
- Created‑at timestamp
- Summary text (tier `SUMMARY`)

---

## Source

- [PARAMS--METADATA-SETUP-PROFILE01](file:///c:/Users/freshair/cognitive_system/gks/params/PARAMS--METADATA-SETUP-PROFILE01.md) — Metadata Menu recommended parameter profile
- [RUNBOOK--OBSIDIAN-LINTER-SETUP](file:///c:/Users/freshair/cognitive_system/gks/runbook/RUNBOOK--OBSIDIAN-LINTER-SETUP.md) — Linter plugin setup (for YAML formatting)
- [atom_schema.yaml](file:///c:/Users/freshair/cognitive_system/atom_schema.yaml) — GKS frontmatter standard schema
- [Metadata Menu Documentation](https://mdelobelle.github.io/metadatamenu) — Official plugin docs

---

*End of Runbook*
