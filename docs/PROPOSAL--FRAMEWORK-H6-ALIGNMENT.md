---
title: PROPOSAL--FRAMEWORK-H6-ALIGNMENT
summary: ข้อเสนอปรับ FRAMEWORK--HIERARCHY-COMPACTION-STANDARDS ให้รองรับ H6 (Context Scaling Tier) ให้ตรงกับ STD-Execution-Governance และ GRL — เสนอเพื่อให้เจ้าของอนุมัติ (docs-only proposal)
doc_id: GVDOC-1003-P1
created: "2026-07-04T00:00:00+07:00,Claude(Agent)"
state: proposal
type: proposal
target_doc: FRAMEWORK--HIERARCHY-COMPACTION-STANDARDS.md (GVDOC-1003, v1.3.0b / 2.0.0)
---

# PROPOSAL: Align FRAMEWORK H-scale with STD/GRL `H6`

> สถานะ: **ข้อเสนอ เพื่อให้เจ้าของพิจารณา** — ยังไม่แก้ไฟล์เป้าหมาย. This is a docs-only proposal; no edits have been applied to `FRAMEWORK--HIERARCHY-COMPACTION-STANDARDS.md`.

---

## 1. Finding — FRAMEWORK carries TWO different H-axes; only ONE is the STD/GRL axis

FRAMEWORK ใช้ตัวอักษร `H` กับ **สองแกนที่ต่างกันโดยสิ้นเชิง** ในไฟล์เดียว:

**Axis A — Compaction Height (§2, "Compaction Heights: H5 - H1")** = *physical file layering depth*, NOT graph hops. Evidence (line 50–54):
> `* **H5 (3 Layers)** ➔ [L2-System] ➔ [L1-Module] ➔ [L0-Function]`
> `* **H1 (8 Layers)** ➔ [L7-System] ➔ ... ➔ [L0-Method]`

Here the H-number is **inverted vs. depth** (H5 = shallowest / 3 layers; H1 = deepest / 8 layers), and each `[L?-?]` is a **file-compaction layer** (System→Module→Function inside one physical file), governed by the Parser Engine in §4 ("State Partitioning… `[L\d-.+]`"). This is a **disk/atom-compaction** axis — it has nothing to do with retrieval hops or governance complexity.

**Axis B — Context Scaling Tier (§3, "H0 … H5")** = *graph hop-depth* — this **IS** the same axis as GRL hops and STD tiers. Evidence (line 60, 62, 82):
> `จำกัดวง (Local Graph Mode) ของ Agent ไว้สูงสุดที่ 5 Hops (รวมตัวมันเอง = 6 Nodes)`
> `H0 - Subtasks / Pull Requests (0 Hop …)` … `H5 - Masterplan / Roadmap (5 Hops …)`

The §3 labels (`H0 Subtask/PR` … `H5 Masterplan/Roadmap`) are **identical** to STD §3 (lines 44–49) and GRL FR1 (lines 20–24). The §3 changelog even states (line 125): *"Formalized Context Scaling Tier (H0-H5) as a native retrieval protocol."*

**Verdict:** For the axis that H6 concerns (retrieval hop-depth / governance tier), FRAMEWORK §3 is the **SAME axis** as STD and GRL — and it currently **stops at H5**, one tier short. FRAMEWORK §2's "Compaction Height" is a **DIFFERENT axis** and must **not** be touched by an H6 change (its H5 is a top-of-range compaction label, unrelated).

---

## 2. Recommendation — Extend §3 to `H6` in its own vocabulary; leave §2 untouched (add a disambiguation note)

**EXTEND (§3 only).** Because §3 is provably the same protocol as STD/GRL and already declares itself a "native retrieval protocol", it should carry the new `H6` tier verbatim, in FRAMEWORK's bilingual house style. The "single-agent ceiling, then decompose" semantic maps cleanly onto §3's existing Small-World `[!TIP]` (line 87), which already says: exceed the ceiling ⇒ the architecture is wrong and must be split — H6 formalizes that as *"decompose across multiple agents"* rather than *"retrieve deeper"*.

**DO NOT extend §2.** §2's "H5 (3 Layers)" is the shallow end of the *compaction* axis, not a hop tier; adding "H6" there would be a category error. Instead add a one-line disambiguation note so future editors don't conflate the two H's.

Rejected alternative (keep-at-H5 + cross-ref only): insufficient — §3 self-identifies as the retrieval protocol and STD/GRL are already at H6, so an unqualified H5 ceiling here is now stale, not merely under-cross-referenced.

---

## 3. Proposed edits (before → after, apply verbatim if approved)

### Edit 1 — §3 intro: state the ceiling is 6 hops with decomposition semantics (line 60)

**BEFORE**
```
ดังนั้นเราจึงสร้าง **"Scaling Tier"** เพื่อจำกัดวง (Local Graph Mode) ของ Agent ไว้สูงสุดที่ **5 Hops (รวมตัวมันเอง = 6 Nodes)** ซึ่งพิสูจน์ได้ทางคณิตศาสตร์แล้วว่าเพียงพอต่อการเข้าถึง Context ทั้งโปรเจกต์โดยไม่ต้องโหลดไฟล์ทั้งหมด:
```

**AFTER**
```
ดังนั้นเราจึงสร้าง **"Scaling Tier"** เพื่อจำกัดวง (Local Graph Mode) ของ Agent ไว้ที่ **สูงสุด 6 Hops (H6 = เพดานแข็งของ Agent ตัวเดียว)** ซึ่งพิสูจน์ได้ทางคณิตศาสตร์แล้วว่าเพียงพอต่อการเข้าถึง Context ทั้งโปรเจกต์โดยไม่ต้องโหลดไฟล์ทั้งหมด งานใดที่ลึกเกิน 6 Hops **ต้องหั่นแบ่งกระจายไปยัง Agent หลายตัว (Decompose)** โดยแต่ละตัวจำกัดรัศมี ≤6 Hops ของตนเอง — ไม่ใช่การไล่สแกนทั้งกราฟ:
```

### Edit 2 — §3 tier list: append the `H6` row after H5 (after line 84)

**BEFORE** (last tier bullet, line 81–84)
```
*   **H5 - Masterplan / Roadmap (5 Hops: Enterprise Vision)** 
    *   **ลักษณะงาน:** ทิศทางและแผนงานระยะยาวระดับองค์กร (Vision & Roadmap) ที่ส่งผลต่อทุกระบบในบริษัท
    *   **บริบทที่ใช้:** `5 Hops` ครอบคลุมฐานความรู้ทั้งหมด (GKS) เพื่อหาจุดกระทบข้ามระบบ (Cross-System Refactoring)
    *   **Workflow:** ดูแลจัดการโดยมนุษย์ (USER) เป็นผู้ควบคุมหลักในการบริหารความเสี่ยง
```

**AFTER** (add the new bullet immediately below)
```
*   **H5 - Masterplan / Roadmap (5 Hops: Enterprise Vision)** 
    *   **ลักษณะงาน:** ทิศทางและแผนงานระยะยาวระดับองค์กร (Vision & Roadmap) ที่ส่งผลต่อทุกระบบในบริษัท
    *   **บริบทที่ใช้:** `5 Hops` ครอบคลุมฐานความรู้ทั้งหมด (GKS) เพื่อหาจุดกระทบข้ามระบบ (Cross-System Refactoring)
    *   **Workflow:** ดูแลจัดการโดยมนุษย์ (USER) เป็นผู้ควบคุมหลักในการบริหารความเสี่ยง
*   **H6 - Full Network / Enterprise Ceiling (6 Hops: เพดานสูงสุดของ Agent ตัวเดียว)** 
    *   **ลักษณะงาน:** การไล่วิเคราะห์ความเกาะเกี่ยวเชิงระบบ (Systemic Coupling) หรือการกู้คืนข้ามระบบที่หายาก — เป็นด่านสุดท้ายของการยกระดับ (Final Escalation Ceiling)
    *   **บริบทที่ใช้:** `6 Hops` = **รัศมีบริบทสูงสุดที่ Agent ตัวเดียวเข้าถึงได้** (NOT a whole-graph scan) เกินจากนี้ห้ามไล่ลึกต่อ
    *   **Workflow:** เมื่อชนเพดาน H6 ให้ **หั่นแบ่งงานกระจายไปยัง Agent หลายตัว** (แต่ละตัวรับผิดชอบรัศมี ≤6 Hops ของตนเอง) ภายใต้การควบคุมความเสี่ยงโดยมนุษย์ (USER)
```

### Edit 3 — §3 `[!TIP]` callout: raise ceiling reference 5→6 (line 87)

**BEFORE**
```
> กฎ 6 Nodes (H0 ถึง H5) คือมาตรฐานที่อ้างอิงจาก **Small World Phenomenon**: หากงานใดในระบบของคุณต้องวิเคราะห์ลึกเกิน 5 Hops เพื่อที่จะเข้าใจความสัมพันธ์ แสดงว่าสถาปัตยกรรมของคุณไม่ได้เป็นแบบ Small World Network แต่เป็น Spaghetti Code ที่มีการผูกขาด (Coupling) ผิดปกติ และจำเป็นต้อง Refactoring ทันที
```

**AFTER**
```
> กฎ 7 Nodes (H0 ถึง H6) คือมาตรฐานที่อ้างอิงจาก **Small World Phenomenon**: H6 (6 Hops) คือ **เพดานแข็งของ Agent ตัวเดียว** — หากงานใดต้องวิเคราะห์ลึกเกิน 6 Hops เพื่อเข้าใจความสัมพันธ์ ห้ามไล่ลึกต่อ ให้ **หั่นแบ่งงานไปยัง Agent หลายตัว (Decompose)**; และหากงานระดับ H1-H5 กลับต้องพึ่ง Hop จำนวนมากผิดปกติ แสดงว่าสถาปัตยกรรมของคุณไม่ได้เป็นแบบ Small World Network แต่เป็น Spaghetti Code ที่มีการผูกขาด (Coupling) ผิดปกติ และจำเป็นต้อง Refactoring ทันที
```

### Edit 4 — §2 heading: add a one-line disambiguation note so §2's H's are not confused with §3's (after line 47)

**BEFORE**
```
## **2. มาตรฐานระดับความลึกการบีบอัดไฟล์ (Compaction Heights: H5 - H1)**
การเลือกใช้งานความสูง (Height) จะเป็นตัวกำหนดว่าใน 1 ไฟล์จะมีการซ้อนทับกันกี่ระดับชั้น โดยแบ่งออกตามความซับซ้อนของแต่ละ System ดังนี้:
```

**AFTER**
```
## **2. มาตรฐานระดับความลึกการบีบอัดไฟล์ (Compaction Heights: H5 - H1)**
การเลือกใช้งานความสูง (Height) จะเป็นตัวกำหนดว่าใน 1 ไฟล์จะมีการซ้อนทับกันกี่ระดับชั้น โดยแบ่งออกตามความซับซ้อนของแต่ละ System ดังนี้:

> [!NOTE]
> **แกนคนละแกนกับ §3.** `H` ในหัวข้อนี้คือ **ความลึกการบีบอัดไฟล์กายภาพ (Compaction Height / on-disk layers)** ตัวเลขวิ่งกลับด้าน (H5 = ตื้นสุด 3 ชั้น … H1 = ลึกสุด 8 ชั้น) — **ไม่ใช่** Context Scaling Hop ใน §3 และ **ไม่เกี่ยวกับ H6** ของ STD/GRL. The `H6` retrieval ceiling applies to §3 only.
```

### Edit 5 — CHANGELOG: add entry in the doc's house style (top of table, after line 121)

**BEFORE**
```
| Version | Date | Status | Summary |
|---|---|---|---|
| 1.3.0b | 2026-06-07 | active | ทำการวิเคราะห์และแยกแกนเนื้องาน (WBS) ออกจากแกนเวลา (Sprint/Cycle) และปรับการแมป H0-H5 ให้ตรงตามมาตรฐาน Agile |
```

**AFTER**
```
| Version | Date | Status | Summary |
|---|---|---|---|
| 2.1.0 | 2026-07-04 | active | ขยาย Context Scaling Tier ใน §3 จาก H0-H5 เป็น **H0-H6** ให้ตรงกับ STD-Execution-Governance §3 และ SPEC--GRAPH-RETRIEVAL-LAYER (H6 = เพดานแข็งของ Agent ตัวเดียว 6 Hops เกินนั้นให้ Decompose); เพิ่มหมายเหตุแยกแกน Compaction Height (§2) ออกจาก Context Hop (§3) อย่างชัดเจน |
| 1.3.0b | 2026-06-07 | active | ทำการวิเคราะห์และแยกแกนเนื้องาน (WBS) ออกจากแกนเวลา (Sprint/Cycle) และปรับการแมป H0-H5 ให้ตรงตามมาตรฐาน Agile |
```

> หมายเหตุเวอร์ชัน: ไฟล์นี้มี changelog สองสายเลข (`1.3.0b` แบบ beta และ `2.0.0` แบบ stable ในแถวล่างสุด). เจ้าของต้องเลือก: ถ้ายึดสาย stable `2.0.0` แนะนำให้ตั้งเวอร์ชันนี้เป็น **`2.1.0`** และอัปเดต frontmatter `version:` (บรรทัด 7) จาก `"1.3.0b"` → `"2.1.0"` พร้อม `updated:` timestamp ให้สอดคล้อง.

---

## 4. Owner decision required

**Owner decision required:** (a) approve extending FRAMEWORK §3 to **H0–H6** (Edits 1–3) plus the §2 disambiguation note (Edit 4); and (b) confirm the changelog/version convention for Edit 5 — bump to **`2.1.0`** on the stable line and update the frontmatter `version:`/`updated:` fields, or keep the `x.x.xb` beta line instead. No edits will be applied to `FRAMEWORK--HIERARCHY-COMPACTION-STANDARDS.md` until approved.
