---
version: "0.1.0"
created_at: "2026-08-30T21:00:00+07:00,Claude Opus 5,working-tree"
last_update: "2026-08-30T21:00:00+07:00,Claude Opus 5"
status: proposed
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

เอกสารนี้ยังไม่ใช่การอนุมัติ ต้องได้รับ approval ก่อนแตะโค้ด (C-2)

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

## 4. รูปร่างที่เสนอ

    CREATE TABLE IF NOT EXISTS edges (
        edge_key   TEXT PRIMARY KEY,   -- hex ของ u128 (ดู §4.1)
        id         TEXT NOT NULL,
        from_u32   INTEGER NOT NULL,
        to_u32     INTEGER NOT NULL,
        rel        TEXT NOT NULL,
        props      TEXT,
        valid_from TEXT NOT NULL,
        valid_to   TEXT,
        impact     REAL,
        caused_by  TEXT,
        clock_time INTEGER NOT NULL,
        clock_peer TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_u32, rel);
    CREATE INDEX IF NOT EXISTS idx_edges_to   ON edges(to_u32, rel);

additive `CREATE TABLE IF NOT EXISTS` ตามแบบเดียวกับ migration v3 ของ
`node_versions` และต้องขึ้น `PROJECTION_SCHEMA_VERSION` เป็น 4

### 4.1 u128 key ไม่พอดีกับ SQLite

`edges: DashMap<u128, EdgeOutput>` แต่ `INTEGER` ของ SQLite คือ i64 —
u128 ใส่ไม่ได้ ทางเลือก:

- **TEXT hex 32 ตัว** — อ่านง่าย debug ง่าย แต่กินที่และเทียบช้ากว่า
- **BLOB 16 ไบต์** — กะทัดรัด เรียงถูกต้อง แต่ดูด้วยตาไม่ได้
- **ใช้ `id` เป็น PK ไปเลย** — `edge_key` derive จาก `id` เสมออยู่แล้ว
  (`trunc128(SHA256(id))`, ADR--GENESISDB-EDGE-NUMERIC-KEYS) จึงไม่ต้องเก็บซ้ำ

ข้อสามดูตรงที่สุด แต่ต้องยืนยันก่อนว่า `id` unique จริงในทุก path

### 4.2 join กลับไปหา node ต้องผ่าน node_versions

`from_u32`/`to_u32` เป็น interned u32 ซึ่ง join กับ `props.node_u32` และ
`node_labels.node_u32` ได้ตรง ๆ — ดี แต่ผู้เรียกคิดเป็น **id string** และ
`props` ไม่มีคอลัมน์ `id` เลย ต้องผ่าน `node_versions.id`

ควรพิจารณา VIEW ที่ทำ join นั้นให้ เพื่อไม่ให้ทุกคนเขียนเองแล้วเขียนผิด

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

- **ขนาด** ฐานตัวอย่างนี้ 15,393 edges → ตาราง + 2 index น่าจะ ~2-4 MB บน
  projection ที่ตอนนี้ 12.3 MB **ยังไม่ได้วัด** ต้องวัดก่อนตัดสินใจ
- **write amplification** ทุก `add_edge` / `retract_edge` เพิ่มการเขียน SQLite
  หนึ่งครั้ง — กระทบ ingestion throughput ที่ `snb-bulk-ingestion` วัดอยู่
  ต้อง bench ก่อน/หลัง
- **backfill** ฐานที่มีอยู่แล้ว (เช่นฐาน RAG นี้) มี edges ใน `edges.bin` แต่ไม่มี
  ใน projection — migration ต้อง backfill ตอน open ครั้งแรก ซึ่งสำหรับ 15k edges
  น่าจะเร็ว แต่ต้องมีขอบเขตที่วัดแล้วสำหรับฐานใหญ่
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

## 8. คำถามที่ต้องการคำตอบก่อนเริ่ม

1. §3.1 ไม่ทำเลยดีกว่าไหม — ช่องว่างจริงคือการผสม traversal กับ aggregation
   ในคำสั่งเดียว คุ้มกับ write amplification + ขนาดหรือเปล่า
2. bitemporal เอาแบบ (ก) ตาราง + `edges_current` view ไหม
3. u128 key เก็บเป็นอะไร — หรือใช้ `id` เป็น PK ไปเลย (§4.1)
4. ต้องวัดขนาดกับ ingestion ก่อนตัดสินใจ หรืออนุมัติให้ลงมือแล้ววัดระหว่างทาง
