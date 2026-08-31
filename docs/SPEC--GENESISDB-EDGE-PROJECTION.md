---
version: "0.1.0"
created_at: "2026-08-30T21:00:00+07:00,Claude Opus 5,working-tree"
last_update: "2026-08-31T02:00:00+07:00,Claude Opus 5"
status: accepted
superseded_by: null
attributes:
  doc_type: "spec"
  domain: "relational-query-surface"
  scope: "projecting edges into projection.sqlite"
  complexity: "C-2"
  risk: "HIGH"
  owner: "Boss (Founder)"
---

# SPEC - Edge Projection

## 1. คำขออนุมัติ

ขออนุมัติ **ฉาย edges ลง `projection.sqlite`** เพื่อให้ read-only SQL surface
(`SPEC--GENESISDB-READONLY-SQL-SURFACE`, ship แล้วใน #160/#161) เขียน query
ที่ join ตามความสัมพันธ์ได้

**อนุมัติแล้ว 2026-08-31** implementation อยู่ใน `projection_apply_edge_tx` +
`projection_backfill_edges` ดู §5 สำหรับต้นทุนที่วัดได้จริง และ §8 สำหรับคำตอบ
ของคำถามทั้งสี่ข้อ

## 2. ปัญหา — วัดจากฐานจริง ไม่ใช่จากการอ่านโค้ด

ลอง `query_sql` กับ RAG store จริง (`genesis_smartgift_store_v4`, สำเนา):

    engine โหลด:  4,701 nodes  ·  15,393 edges
    SQL เห็น:     7 ตาราง — props, node_labels, node_versions,
                  projection_state, relational_schema_registry,
                  applied_relational_mutations, applied_transactions
    edges ที่ SQL เห็น: 0

    SELECT ... FROM edges  ->  SQL_REJECTED: no such table: edges

`grep "INSERT.*edges\|CREATE TABLE.*edges" src/lib.rs` ไม่มีผลลัพธ์ —
**projection เป็น node-only โดยโครงสร้าง ไม่ใช่เพราะยังไม่ได้เขียน**

ผลที่ตามมาเป็นรูปธรรม: ใน 5 ProductModel ที่สุ่มมา มี 37 ความสัมพันธ์ที่
`neighbors()` เห็นแต่ SQL ไม่เห็น คำถามอย่าง

> ProductModel ตัวไหนมี CatalogOffer มากที่สุด

**เขียนไม่ได้เลย** ไม่ใช่เขียนได้แต่ช้า ไม่ใช่เขียนได้แต่อ้อม — ข้อมูลไม่อยู่ใน
projection สำหรับ catalog ที่มีชั้น SKU → Variant → Offer นี่คือรายงานเกือบทั้งหมด

ย้อนกลับไปที่เหตุผลตั้งต้นของ `query_sql` ใน
`SPEC--GENESISDB-READONLY-SQL-SURFACE` §2: "งาน report/analytics ทำใน engine
ไม่ได้ ต้อง export ออกไปข้างนอก ซึ่งเปิดประตูสู่ dual-write" — ตอนนี้เหตุผลนั้น
ยังจริงอยู่ครึ่งหนึ่ง งานที่แตะความสัมพันธ์ยังต้อง export

## 3. ทางเลือกที่ไม่ทำ และเหตุผล

**3.1 ไม่ทำอะไรเลย** — ใช้ `neighbors()` / HQL `TRAVERSE` / `MATCH` แทน

นี่เป็นทางเลือกที่ **จริงจัง** ไม่ใช่ฟาง: graph API ตอบคำถามกราฟได้อยู่แล้ว และ
ตอบได้เร็วกว่า SQL ที่ต้อง recursive join ด้วย ช่องว่างที่แท้จริงแคบกว่าที่เห็น
— มันคือการ **ผสม** traversal เข้ากับ aggregation ใน query เดียว เช่น
"นับ offer ต่อ model แล้วเรียง" ซึ่งวันนี้ต้องดึงออกมา loop ใน host language

ถ้าคำตอบคือ "ผสมได้ไม่คุ้มกับต้นทุนข้างล่าง" ให้ปิดข้อเสนอนี้แล้วบันทึกไว้ใน §8
ของสเปกเดิมว่าเป็นขีดจำกัดถาวรที่ตั้งใจ

**3.2 ให้ HQL compile ลง SQL** — งานคนละชิ้น ใหญ่กว่ามาก และยังต้องมี edges ใน
projection อยู่ดี ไม่ได้แทนกัน

**3.3 ให้ `query_sql` อ่าน `edges.bin` ผ่าน virtual table** — เลี่ยงการ duplicate
ข้อมูล แต่ต้องเขียน vtab module เอง และผูก lifetime ของ read-only connection
เข้ากับ live map ที่เขียนอยู่ ซับซ้อนกว่าและอันตรายกว่าการฉายลงตาราง

## 4. รูปร่างที่สร้างจริง

    CREATE TABLE IF NOT EXISTS edges (
        id         TEXT PRIMARY KEY,   -- ไม่ใช่ edge_key ดู §4.1
        from_u32   INTEGER NOT NULL,
        to_u32     INTEGER NOT NULL,
        from_id    TEXT NOT NULL,      -- เพิ่มจากที่เสนอ ดู §4.2
        to_id      TEXT NOT NULL,
        rel        TEXT NOT NULL,
        props      TEXT NOT NULL DEFAULT '{}',
        valid_from TEXT NOT NULL,
        valid_to   TEXT,
        recorded_at   TEXT NOT NULL DEFAULT '',
        superseded_by TEXT,
        impact     REAL,
        caused_by  TEXT,
        clock_time INTEGER NOT NULL DEFAULT 0,
        clock_peer TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_u32, rel);
    CREATE INDEX IF NOT EXISTS idx_edges_to   ON edges(to_u32, rel);
    CREATE VIEW  IF NOT EXISTS edges_current AS
        SELECT * FROM edges WHERE valid_to IS NULL;

additive `CREATE TABLE IF NOT EXISTS` ตามแบบเดียวกับ migration v3 ของ
`node_versions` และขึ้น `PROJECTION_SCHEMA_VERSION` เป็น 4

ต่างจากที่เสนอไว้สามจุด: ใช้ `id` เป็น PK แทน `edge_key` (§4.1),
เพิ่ม `from_id`/`to_id` (§4.2) และเก็บ `recorded_at`/`superseded_by` ที่ร่างแรก
ตกไป — `EdgeOutput` มีสองฟิลด์นั้น การฉายแบบขาดฟิลด์คือ projection ที่ทำให้
คำถามบางข้อตอบไม่ได้โดยไม่มีเหตุผล

### 4.1 u128 key ไม่พอดีกับ SQLite

`edges: DashMap<u128, EdgeOutput>` แต่ `INTEGER` ของ SQLite คือ i64 —
u128 ใส่ไม่ได้ ทางเลือก:

- **TEXT hex 32 ตัว** — อ่านง่าย debug ง่าย แต่กินที่และเทียบช้ากว่า
- **BLOB 16 ไบต์** — กะทัดรัด เรียงถูกต้อง แต่ดูด้วยตาไม่ได้
- **ใช้ `id` เป็น PK ไปเลย** — `edge_key` derive จาก `id` เสมออยู่แล้ว
  (`trunc128(SHA256(id))`, ADR--GENESISDB-EDGE-NUMERIC-KEYS) จึงไม่ต้องเก็บซ้ำ

**เลือกข้อสาม** ยืนยันแล้วว่า `id` เป็น identity จริง: live map คือ
`DashMap<u128, EdgeOutput>` ที่ key มาจาก `edge_key(id)` เสมอ การชนกันของ `id`
จึงเป็นบั๊กที่มีอยู่แล้วก่อนหน้านี้ ไม่ใช่ความเสี่ยงที่การเลือกนี้สร้างขึ้นใหม่

### 4.2 join กลับไปหา node ต้องผ่าน node_versions

`from_u32`/`to_u32` เป็น interned u32 ซึ่ง join กับ `props.node_u32` และ
`node_labels.node_u32` ได้ตรง ๆ — ดี แต่ผู้เรียกคิดเป็น **id string** และ
`props` ไม่มีคอลัมน์ `id` เลย ต้องผ่าน `node_versions.id`

**แก้ด้วยการเก็บทั้งสองสะกด** `from_id`/`to_id` อยู่ในตารางเลย ไม่ต้องมี VIEW
เพิ่ม — ถ้าไม่มี string ทุก query จะต้องวิ่งผ่าน `node_versions` เพียงเพื่อเรียก
ชื่อ endpoint ของตัวเอง ต้นทุนคือความซ้ำซ้อนของข้อมูลใน projection ที่สร้างใหม่
ได้อยู่แล้ว ซึ่งถูกกว่าการให้ทุกคนเขียน join สามทางเอง

### 4.3 bitemporal — ข้อที่ต้องตัดสินใจ ไม่ใช่ข้อที่ต้อง implement

`retract_edge` เป็น soft-delete: ตั้ง `valid_to` แล้วเดินนาฬิกา edge ยังอยู่ใน
log และ time-travel เห็น แต่ `neighbors` ซ่อนจาก current view

ถ้าฉายทั้งหมดลงตารางเดียว **`SELECT * FROM edges` จะรวม edge ที่ retract แล้ว** —
ต่างจากที่ `neighbors()` ตอบ คนที่เคยชินกับ graph API จะได้คำตอบผิดโดยไม่รู้ตัว
ซึ่งเป็น "คำตอบผิดที่หน้าตาเหมือนถูก" แบบเดียวกับที่ #160 ไล่ปิดไปสามจุด

ทางเลือก: (ก) ฉายทั้งหมด + VIEW `edges_current` ที่กรอง `valid_to IS NULL`,
(ข) ฉายเฉพาะ current แล้วเสียความสามารถ time-travel ใน SQL,
(ค) ฉายทั้งหมดแล้วบังคับให้ต้องเลือก view ไม่มี default

**ข้อเสนอ: (ก)** — เก็บความจริงทั้งหมด แต่ให้ทางที่ถูกอยู่ใกล้มือ

## 5. ต้นทุนที่ต้องยอมรับ

**วัดแล้ว** — 40,000 edges ผ่าน `bulk_add_edges`, release build, n=3 ต่อฝั่ง
ค่าที่รายงานคือมัธยฐาน:

| สถานะ | us/edge | edges/sec | ทั้งสามรอบ |
|---|---|---|---|
| ก่อน (node-only) | **91** | ~11,000 | 87.6 · 100.3 · 91.1 |
| หลัง ไม่มี secondary index | 142 | ~7,040 | 134.1 · 177.1 · 142.0 |
| หลัง + 2 index (ที่เลือก) | **240** | ~4,160 | 240.5 · 252.2 · 225.3 |

**2.6× ไม่ใช่ 4×** — เลข 4× ที่เห็นตอนแรกมาจากรันเดียวต่อฝั่ง และฐานที่ใช้บังเอิญ
เป็นค่าเร็วสุดของการกระจาย ต้อง n=3 ถึงเห็น

### 5.1 แก้ตัวเลขข้างบน — 2.6× เป็นกรณีดีที่สุด ไม่ใช่ค่าปริยาย

วัดซ้ำด้วย `snb-bulk-ingestion` หลังใส่การจับเวลาเฟส edge พบว่าตารางข้างบน
**วัดกรณีที่เอื้อโดยไม่ได้ตั้งใจ** bench นั้นตั้ง id เองเป็น `e0, e1, ...`
ซึ่งเรียงกัน แต่ค่าปริยายของ API คือ `id: None` ซึ่ง engine สร้าง
`Uuid::new_v4()` ให้ — สุ่มล้วน `edges` มี TEXT primary key การแทรกแบบเรียงจึง
ต่อท้าย B-tree ส่วนแบบสุ่มกระจายทั่วทั้งต้น

แยกตัวแปรเดียว build เดียวกัน harness เดียวกัน n=8 ต่อฝั่ง ไม่มีช่วงทับกัน:

| กรณี | us/edge | เทียบฐาน |
|---|---|---|
| ไม่มี edge projection | ~50 (40.6-60.2, n=10) | — |
| มี + id เรียง (`SNB_EDGE_IDS=seq`) | ~141 (134.8-152.1) | **2.8×** |
| มี + id สุ่ม (ค่าปริยาย) | ~343 (310.0-377.6) | **6.9×** |

2.8× ยืนยันการวัดเดิม 2.6× สำหรับ id เรียง — การวัดนั้นไม่ผิด แต่มันตอบคำถาม
ที่แคบกว่าที่ผมคิด **ต้นทุนที่ caller ทั่วไปจ่ายคือ 6.9×**

ผมไม่ได้เลือก id เรียงเพราะอยากให้ตัวเลขดูดี — เลือกเพราะเขียนง่ายที่สุด แล้ว
ไม่ได้ถามว่าการเลือกนั้นกระทบสิ่งที่วัดไหม **ตัวเลือกที่สะดวกที่สุดกลายเป็น
ตัวเลือกที่เอื้อที่สุดโดยบังเอิญ** และมองไม่เห็นจนมีคนวัดด้วยวิธีอื่น

**ใช้ได้จริง:** caller ที่ส่ง edge id เรียงมาเอง จ่ายต้นทุน projection ไม่ถึง
ครึ่งของค่าปริยาย

แยกส่วนได้ว่า **index แพงกว่าตัว INSERT**: +51 us เป็นตัว INSERT เอง (91→142)
และ +98 us เป็น secondary index สองตัว (142→240) เลือกเก็บ index ไว้เพราะ
projection นี้มีไว้เพื่อ query — ตัดทิ้งคือบั่นทอนเหตุผลที่มันมีอยู่

สมมติฐานสองข้อที่ **ผิด** และวัดจนตกไป (บันทึกไว้กันคนถัดไปเดินซ้ำ):
`prepare_cached` ไม่ช่วยเลย (225-252 us เท่าเดิม) และไม่มี fsync ต่อ edge เพราะ
`execute_batch` เขียนทั้ง chunk ใน transaction เดียวอยู่แล้ว

**ผลกระทบต่อ use case จริง** ซึ่งเป็นหน่วยที่ถูกกว่าอัตราส่วน:

- rebuild ฐาน RAG (15,393 edges): 1.40 s → 3.69 s = **+2.3 วินาที** บน pipeline
  ที่ embed 4,701 nodes ด้วย bge-m3 อยู่แล้ว
- เขียนทีละ edge (per-op path): path นั้น fsync ทุกครั้งที่ **58.8 ms/edge**
  projection เพิ่ม ~149 us = **0.25%** มองไม่เห็น
- **ดิสก์ วัดจากฐานจริง**: projection.sqlite 11.7 → 15.6 MB = **+3.9 MB (+33%)**
  หรือ +2.4% ของทั้ง store (163 MB) เนื้อคอลัมน์ดิบ 2.64 MB ที่เหลือคือ
  row overhead + index สามตัว
- ที่จะเจ็บคือ bulk load หลักล้าน: 1M edges 91 s → 240 s

**`snb-bulk-ingestion` เฝ้าเรื่องนี้ไม่ได้ตอนนั้น และตอนนี้ได้แล้ว** — เดิมมันไม่จับ
เวลาเฟส edge เลย และต่อให้จับจากข้างนอก 4,999 edges ก็ให้ signal/noise = 0.32
แก้แล้วในงานแยก: จับเวลาเฟส edge + ยกเป็น 40,000 ซึ่งพิสูจน์ด้วยการปลูก
จุดบกพร่องว่าแยกสองฝั่งได้เด็ดขาด (40.6-60.2 เทียบ 326.6-431.2 us/edge
ไม่มีช่วงทับกัน)

- **backfill** ฐานที่มีอยู่แล้วมี edges ใน `edges.bin` แต่ไม่มีใน projection —
  วัดจากฐานจริง: backfill 15,393 edges รวมกับการเปิดฐานทั้งหมด 6.6 วินาที
- **WAL ยังเป็นเจ้าของความจริง** projection เป็น derived rebuildable ตามเดิม
  ข้อนี้ไม่เปลี่ยน และเป็นเหตุผลที่ backfill ทำได้อย่างปลอดภัย

## 6. แผนทดสอบ

ทุกข้อพิสูจน์ด้วยการรัน และทุก guard พิสูจน์ด้วยการ **ปลูกจุดบกพร่อง**:

1. เพิ่ม edge → ปรากฏใน `edges` ทันทีที่ commit
2. `retract_edge` → `valid_to` ถูกตั้ง แถวยังอยู่ และหายจาก `edges_current`
3. `edges_current` ให้ผลตรงกับ `neighbors()` **บนฐานเดียวกัน ชุดเดียวกัน** —
   ข้อนี้สำคัญที่สุด เพราะสองผิวที่ตอบคนละอย่างแย่กว่าผิวเดียวที่ตอบไม่ได้
4. backfill: เปิดฐานเก่าที่ไม่มีตาราง → edges ครบตามจำนวนใน `edges.bin`
5. rebuild projection จากศูนย์ → ได้ผลเท่าเดิม (derived จริง)
6. bench ingestion ก่อน/หลัง — ตัวเลขจริง ไม่ใช่ "ไม่น่าจะกระทบ"
7. คำถามที่เป็นต้นเหตุ ตอบได้ในคำสั่งเดียว:

       SELECT m.id, count(*) AS offers
         FROM edges_current e
         JOIN node_versions m ON m.node_u32 = e.from_u32
         JOIN node_labels  lm ON lm.node_u32 = e.from_u32 AND lm.label = 'ProductModel'
         JOIN node_labels  lo ON lo.node_u32 = e.to_u32   AND lo.label = 'CatalogOffer'
        GROUP BY m.id ORDER BY offers DESC LIMIT 10

## 7. สิ่งที่ยังไม่ครอบคลุม

- ไม่แตะ REST — เหมือนเดิม ผิวที่เข้าถึงจากเครือข่ายเป็นการตัดสินใจแยก
- ไม่แยกสิทธิ์ต่อผู้เรียก — เพิ่ม edges คือเพิ่มสิ่งที่ผิวนี้อ่านได้ทั้งหมด
  ให้มากขึ้น ข้อยอมรับใน §5 ของสเปกเดิมขยายตาม
- ไม่ทำ variable-length path ใน SQL — recursive CTE ทำได้แต่ช้ากว่า
  `TRAVERSE` มาก ไม่ใช่เป้าหมายของงานนี้

## 8. คำถามก่อนเริ่ม และคำตอบที่ได้

1. **ทำ** — ตัดสินใจ 2026-08-31 ช่องว่างคือการผสม traversal กับ aggregation
   ในคำสั่งเดียว และมันคุ้ม: ต้นทุนจริงบนงานที่ใช้อยู่คือ +2.3 วินาที กับ +3.9 MB
   (§5) แลกกับรายงานที่เดิมเขียนไม่ได้เลย พิสูจน์ทันทีหลัง merge — matrix query
   ที่วินิจฉัยโครงสร้าง catalog ทั้งชุดรันใน 64 ms
2. **(ก)** ตาราง `edges` เก็บครบ + view `edges_current` ตามที่เสนอ
   `edges_current_agrees_with_the_graph_api` เป็นเทสต์ที่รับน้ำหนักของข้อนี้
3. **ใช้ `id` เป็น PK** เพราะ `edge_key` derive จาก `id` เสมอและไม่เคยถูกเก็บ
   (`trunc128(SHA256(id))`) — `id` คือ identity อยู่แล้ว เก็บ hash ซ้ำคือการ
   สะกดข้อเท็จจริงเดิมด้วยวิธีที่อ่อนกว่า และ SQLite ไม่มีชนิดที่กว้างพอ
4. **วัดก่อน merge** ตัวเลขทั้งหมดอยู่ใน §5 รวมถึงข้อที่วัดแล้วพบว่า
   `snb-bulk-ingestion` เฝ้าเรื่องนี้ไม่ได้
