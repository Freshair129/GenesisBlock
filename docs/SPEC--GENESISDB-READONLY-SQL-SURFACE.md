---
version: "0.1.0b"
created_at: "2026-08-29T23:55:00+07:00,Claude Opus 5,working-tree"
last_update: "2026-08-30T12:00:00+07:00,Claude Opus 5"
status: accepted
superseded_by: null
attributes:
  doc_type: "spec"
  domain: "relational-query-surface"
  scope: "read-only SQL over projection.sqlite"
  complexity: "C-2"
  risk: "HIGH"
  owner: "Boss (Founder)"
---

# SPEC - Read-only SQL Surface

## 1. คำขออนุมัติ

ขออนุมัติเปิด **read-only SQL surface** บน `projection.sqlite` ตามที่
`docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md` §2.2 ข้อ 5 เปิดช่องไว้แล้ว
("Read-only SQL access may be exposed later - a diagnostics/query surface")

**อนุมัติแล้ว 2026-08-30** (C-2 ต้องอนุมัติเอกสารก่อนแตะโค้ด) implementation
อยู่ใน `Storage::query_sql` + NAPI wrapper ดู §7 สำหรับสิ่งที่การวัดจริงเปลี่ยนไป
จากแผนเดิม

## 2. ปัญหา

`RelationalQuery` เป็น IR ที่จงใจให้แคบ:

    RelationalQuery { namespace, table, columns[], joins[], filters[], limit }
    RelationalFilter { column, value }        // constructor เดียวคือ equal()

ทำได้: SELECT columns / JOIN (inner,left) / WHERE col = value / LIMIT

ทำไม่ได้: `>` `<` `LIKE` `IN` `BETWEEN` / `OR` / `GROUP BY` / aggregate /
`ORDER BY` / subquery

ข้อจำกัดอยู่ใน **โครงสร้างข้อมูล** ไม่ใช่แค่ยังไม่ได้ทำ - `RelationalFilter`
มีแค่สองฟิลด์

แต่ ADR §2.1 ระบุว่า SQLite เป็นผู้รับผิดชอบ "Filtering (WHERE incl. OR/parens),
aggregation (count, future group-by)" และตั้งใจให้ HQL compile ลงไปหามัน
**ความสามารถอยู่ในเครื่องแล้ว ที่ขาดคือทางเรียก**

ผลกระทบจริง: งาน report/analytics ทำใน engine ไม่ได้ ต้อง export ออกไปข้างนอก
ซึ่งเปิดประตูสู่ dual-write ที่ ADR ทั้งฉบับมีไว้เพื่อป้องกัน

## 3. ขอบเขต

**ใน scope**
- เมธอดใหม่บน `Storage` + NAPI: รับ SQL string + bound parameters คืน rows
- บังคับ read-only ที่ระดับ SQLite ไม่ใช่ระดับ parse ข้อความ
- ขีดจำกัดเวลา/จำนวนแถว

**นอก scope (จงใจ)**
- **ไม่มี REST route ใน slice นี้** SQL ที่เข้าถึงได้จากเครือข่ายเป็นผิวโจมตี
  คนละระดับกับ in-process ควรตัดสินใจแยกหลังเห็น in-process ทำงานจริง
  (หมายเหตุ: ขัดกับธรรมเนียม "wire ทั้ง NAPI และ REST" ของ CLAUDE.md โดยตั้งใจ)
- ไม่มีการเขียนทุกชนิด
- ไม่แตะ HQL - การให้ HQL compile ลง SQL เป็นงานคนละชิ้น

## 4. การบังคับ read-only

**สี่ชั้น ทุกชั้นตรวจแล้วว่ามีอยู่จริงใน rusqlite 0.32.1**

| ชั้น | กลไก | ยืนยัน |
|---|---|---|
| 1. connection | `OpenFlags::SQLITE_OPEN_READ_ONLY` แยก connection ต่างหาก | `open_projection()` มีอยู่แล้ว `src/lib.rs:3189` |
| 2. authorizer | `Connection::authorizer()` ปฏิเสธทุก action ที่ไม่ใช่การอ่าน | `hooks/mod.rs:405` |
| 3. limits | `set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)` ทำให้ ATTACH เป็นไปไม่ได้ | `limits.rs:62` |
| 4. เวลา | `progress_handler()` + `InterruptHandle` ตัด query ที่เกินงบเวลา | `hooks/mod.rs:395`, `lib.rs:1011` |

ชั้น 1 คือชั้นที่แข็งที่สุด - SQLite ปฏิเสธการเขียนเอง ไม่ต้องเชื่อว่าเราแยกแยะ
SELECT ออกจาก INSERT ได้ถูก ชั้น 2-4 กันสิ่งที่ read-only connection ยังทำได้อยู่

feature ที่ต้องเปิดใน `Cargo.toml`: `rusqlite = { features = ["bundled",
"hooks", "limits"] }` - ทั้ง `hooks` และ `limits` เป็น `[]` เปล่าใน rusqlite
ไม่ดึง dependency เพิ่มเลย

## 5. Threat model

| ผู้เรียกลอง | สิ่งที่หยุด | ถ้าชั้นนั้นพัง |
|---|---|---|
| `INSERT` / `UPDATE` / `DELETE` / `DROP` | ชั้น 1 (SQLite ปฏิเสธ) | ชั้น 2 ปฏิเสธซ้ำ |
| `ATTACH 'other.db'` | ชั้น 3 (limit = 0) | ชั้น 2 ปฏิเสธ action `Attach` |
| `PRAGMA` ที่เปลี่ยนสถานะ | ชั้น 2 | ชั้น 1 กันเฉพาะที่เขียนจริง |
| cartesian join / recursive CTE ไม่รู้จบ | ชั้น 4 + row cap | ไม่มี - ต้องพึ่งชั้นนี้ |
| อ่านตารางภายใน (`relational_schema_registry`) | **ไม่หยุด** | ยอมรับ: เป็น diagnostics surface |
| SQL injection จากค่าที่ผู้ใช้ป้อน | bound parameters เท่านั้น ห้ามต่อสตริง | - |

**สิ่งที่ยอมรับอย่างเปิดเผย:** ผิวนี้อ่านได้ทั้งฐาน รวมถึง props ของทุก node
ถ้าแอปมีข้อมูลที่ผู้เรียกบางกลุ่มไม่ควรเห็น ผิวนี้ไม่ได้แยกให้

## 6. ขีดจำกัด (ค่าเริ่มต้น ปรับได้ต่อ query)

- เวลา: 5 วินาที
- จำนวนแถว: 10,000
- ขนาดผลลัพธ์: 32 MB

เกินขีด = error ไม่ใช่การตัดผลลัพธ์เงียบ ๆ ผลลัพธ์ที่ถูกตัดโดยไม่บอกคือคำตอบผิด
ที่ดูเหมือนถูก

## 7. แผนทดสอบ

ทุกข้อพิสูจน์ด้วยการ **รัน** ไม่ใช่การอ่านโค้ด และทุกข้อต้อง fail closed:

1. `SELECT` ปกติ - คืนแถวถูกต้อง
2. `INSERT` / `UPDATE` / `DELETE` / `DROP TABLE` / `CREATE TABLE` - ปฏิเสธทุกตัว
3. `ATTACH DATABASE` - ปฏิเสธ
4. `PRAGMA journal_mode = DELETE` - ปฏิเสธ
5. query ที่วนไม่จบ - ถูกตัดที่ขีดเวลา ไม่ค้าง
6. ผลลัพธ์เกิน row cap - error ไม่ใช่ตัดเงียบ
7. **หลังทุกเคสข้างบน ฐานยังอ่านได้และเนื้อหาไม่เปลี่ยน** - ข้อนี้สำคัญที่สุด
   เพราะ "ถูกปฏิเสธ" กับ "เขียนไปแล้วค่อย error" แยกกันไม่ออกถ้าไม่ตรวจ
8. bound parameter ที่มี `'; DROP TABLE x; --` ถูกปฏิบัติเป็นค่า ไม่ใช่ SQL

**เพิ่มระหว่าง implement** — สองข้อนี้ไม่ได้อยู่ในแผนเดิม เจอจากการวัดพฤติกรรมจริง
ของ implementation ที่เขียนเสร็จแล้ว ทั้งคู่ไม่ใช่ช่องโหว่ด้านความปลอดภัย
(ไม่มีอะไรถูกเขียนเพิ่ม) แต่เป็น **คำตอบผิดที่หน้าตาเหมือนถูก** ชนิดเดียวกับ
ผลลัพธ์ที่ถูกตัดเงียบ ๆ ในข้อ 6 จึงใช้มาตรฐานเดียวกัน คือ error ไม่ใช่เงียบ:

9. หลาย statement ในครั้งเดียว — SQLite compile แค่ statement แรกแล้วทิ้งที่เหลือ
   เงียบ ๆ `SELECT ...; DROP TABLE ...` เคยคืน `Ok` พร้อมแถวของ SELECT (วัดแล้ว:
   DROP ไม่ได้รัน ตารางยังอยู่ครบ) ตอนนี้ปฏิเสธ พร้อมเคสตรงข้ามที่ต้อง **ไม่**
   ถูกปฏิเสธ — `;` ที่อยู่ใน string / comment / quoted identifier และข้อความไทย
   (คุม byte scan กับ UTF-8 หลายไบต์)
10. ชื่อคอลัมน์ผลลัพธ์ซ้ำกัน — JSON object เก็บ key ซ้ำไม่ได้ วัดก่อนใส่ guard:
    `SELECT id, price AS id` คืน `{"id": <ราคา>}` คือ **ค่าของคอลัมน์ที่สอง
    ใต้ชื่อของคอลัมน์แรก** ไม่ใช่แค่ field หาย แต่เป็นค่าผิดใต้ชื่อที่ถูก
11. input ที่ไม่มี statement (ว่าง / มีแต่ comment / `;` เปล่า) — เดิมล้มอยู่แล้ว
    แต่ด้วยข้อความ `not an error` ของ SQLite ตอนนี้บอกเหตุผลตรง ๆ

ทุก guard ในแผนนี้พิสูจน์ด้วยการ **ปลูกจุดบกพร่อง** แล้วดูว่าเทสต์แดง ไม่ใช่
ดูว่าเทสต์เขียว: ถอด authorizer → แดง 2 ตัว, ถอด tail check + dup check → แดง 2 ตัว

## 8. สิ่งที่ยังไม่ครอบคลุม

- ไม่มี REST route จึงยังพิสูจน์ไม่ได้ว่าปลอดภัยพอสำหรับผิวที่เข้าถึงจากเครือข่าย
- ไม่มีการแยกสิทธิ์ต่อผู้เรียก - ทุกคนที่เรียกได้ อ่านได้หมด
- `projection.sqlite` เป็น projection ที่ rebuild ได้ ผลลัพธ์จึงสะท้อนสถานะหลัง
  apply แล้วเท่านั้น ไม่ใช่ WAL ที่ยังไม่ถูก apply

## 9. คำถามก่อนเริ่ม และคำตอบที่ได้

ทั้งสามข้ออนุมัติตามที่เสนอ 2026-08-30:

1. **ไม่** เปิด REST ใน slice แรก — ตกลง ผิวที่เข้าถึงจากเครือข่ายเป็นการตัดสินใจ
   แยก ควรทำหลังเห็น in-process ทำงานจริง
2. ยอมรับว่าผิวนี้อ่านได้ทั้งฐานโดยไม่มีการแยกสิทธิ์ — บันทึกไว้ใน §5 และ §8
3. ขีดจำกัด 5 วิ / 10,000 แถว / 32 MB ใช้ตามที่เสนอ
