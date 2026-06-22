---
proposed_id: METRICS-REVIEW--2026-06-22-WEEKLY
type: metrics-review
status: complete
period: week ending 2026-06-22
data_source: benchmark P15–P30 (session 2026-06-21)
related:
  - REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
  - AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG
  - AUDIT--P22-GRAPH-TRAVERSAL
  - AUDIT--P23-NEO4J-HEAD-TO-HEAD
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - AUDIT--P27-LANCEDB-HEAD-TO-HEAD
  - AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD
  - AUDIT--P29-ROCKSDB-GRAPH-HEAD-TO-HEAD
  - AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD
---

# GenesisBlock — Weekly Metrics Review
## สัปดาห์สิ้นสุด 22 มิถุนายน 2026

---

## สรุป

สัปดาห์นี้เป็นสัปดาห์แรกที่ GenesisBlock มีตัวเลข **วัดจริงทุกตัว** แทนตัวเลขจาก
spec ผู้ผลิต ผลออกมาดีกว่าที่คาด: query latency ใกล้เคียง Chroma ทั้งที่ GenesisBlock
persistent ทุก write ด้าน graph traversal ชนะ LadybugDB (คู่แข่ง on-niche ที่สุด)
ถึง **168×** บน hop1 Ingest throughput กระโดดขึ้น **+668%** จากการแก้ 3 จุดพร้อมกัน
จุดที่ต้องจับตาคือ RAM ceiling (~12.6 GB ที่ 1M nodes) และ ingest ที่ยังแพ้ columnar
system ~60×

---

## North Star

> **Competitive Latency Position** — GenesisBlock ชนะหรือใกล้เคียง embedded competitor
> ที่ใกล้นิชที่สุด ในทุก dimension ที่วัดได้ ด้วยตัวเลขจริง

---

## Metric Scorecard

### Vector Performance (100k vectors, 1024-dim, bge-m3, L2)

| Metric | ค่าสัปดาห์นี้ | สัปดาห์ก่อน | เปลี่ยน | Target | Status |
|---|---|---|---|---|---|
| Query p50 (ef=200) | 974 µs | — (วัดครั้งแรก) | — | <1,000 µs | ✅ On track |
| Query p95 (ef=200) | 1,472 µs | — | — | <2,000 µs | ✅ On track |
| Recall@10 (ef=200) | 0.979 | — | — | ≥0.975 | ✅ On track |
| Bulk insert | ~1,950 vec/s | 254 vec/s | **+668%** | ≥1,000 vec/s | ✅ ชนะ target |
| Concurrent ingest | 839 TPS | 136 TPS | **+515%** | ≥500 TPS | ✅ ชนะ target |
| RAM @5k×1536 | 82 MB | 147 MB | **−44%** | <100 MB | ✅ On track |

### Graph Performance (100k nodes / 800k edges, fanout-8)

| Metric | GenesisBlock | Competitor ใกล้ที่สุด | GenesisBlock vs | Status |
|---|---|---|---|---|
| hop1 p50 | **21.6 µs** | LadybugDB 3,637 µs | **168× faster** | ✅ Strong |
| hop3 p50 | **2.33 ms** | LadybugDB 15.6 ms | **6.7× faster** | ✅ Strong |
| hop6 p50 | **4.40 ms** | DuckDB+graph 4.67 ms | **~tied** | ⚠️ Watch |
| hop1 scale (1M nodes) | 35.4 µs | — | O(neighborhood) ✅ | ✅ Proven |
| RAM @1M/8M | 12.6 GB | Kuzu ~1.1 GB | **11× สูงกว่า** | 🔴 At risk |

### Competitive Benchmark Coverage

| Competitor | Category | วัดแล้ว | ผลสรุป |
|---|---|:---:|---|
| Chroma | embedded vector | ✅ P15, P21 | query parity (974 vs 990 µs); GenesisBlock durable, Chroma in-memory |
| Qdrant | server vector | ✅ P20 | GenesisBlock **3.4× faster** — embedded vs server tax |
| LanceDB | embedded vector | ✅ P27 | GenesisBlock **9× faster** point query; LanceDB trades latency for disk-scale |
| Neo4j | server graph | ✅ P23 | GenesisBlock **7–185× faster** — embedded vs server + JVM tax |
| Kuzu | embedded graph | ✅ P26 | GenesisBlock hop1 **7–166× faster**; Kuzu ingest **60×** เร็วกว่า, memory **11×** น้อยกว่า |
| DuckDB+graph | embedded graph | ✅ P28 | GenesisBlock hop1 **54×**; tied hop6 (4.40 vs 4.67 ms) |
| RocksDB+graph | embedded KV | ✅ P29 | hop1 **tied** (21.6 vs 26.8 µs) — validates GenesisBlock architecture |
| LadybugDB | embedded graph+vec | ✅ P30 | GenesisBlock **168× / 6.7× / 13.5×** (hop1/3/6); no payload caveat |

Coverage: **8/8 — ครบทุกเจ้าที่ named ✅**

### Engineering Health

| Metric | ค่า | Status |
|---|---|---|
| Rust test suite | 20 passed / 0 failed / 22 binaries | ✅ Green |
| Correctness issues fixed (session) | 4 | ✅ Closed |
| Performance optimizations shipped | 5 (P-B, #1, #2, #3, ef) | ✅ |
| Governance overhead | <0.1% (~524 ns/op) | ✅ Negligible |
| K-Impact incremental vs full | up to **398,000×** faster (P25) | ✅ Proven |

---

## Trend Analysis

### 🚀 Ingest Throughput: +668% single-thread, +515% concurrent

สาเหตุ: 3 fixes คู่ขนาน รันพร้อมกันใน session เดียว

| Fix | ผล |
|---|---|
| #1 ถอด global HNSW write lock | 136 → 839 TPS (+515%) |
| #2 batch WAL (1 fsync/chunk) | 254 → 385 vec/s (+52%) |
| #3 parallel rayon HNSW build | 385 → ~1,950 vec/s (+406%) |

ประเมิน: เป็น step-change จาก architectural fix ไม่ใช่ gradual improvement ตัวเลขจะ
stable ที่นี่จนกว่าจะมี optimization รอบถัดไป เช่น deferred/async indexing

### 📉 RAM: −44% (147 → 82 MB @5k×1536)

สาเหตุ: ตัด redundant in-memory f64 embedding copy ออก (P-B, `insert_node_lean`)
ประเมิน: quick win หมดแล้ว low-hanging fruit รอบแรกเสร็จ RAM lever ถัดไปคือ edge UUID
interning — ยากกว่าแต่ impact สูงกว่ามาก (อาจลดได้ถึง 11×)

### 🔴 RAM ceiling ที่ 1M nodes: ~12.6 GB

ตอนนี้ 1M nodes + application overhead อาจ OOM บน 32 GB machine ยังไม่วิกฤต
(100k–500k ใช้งานได้ดี) แต่จำกัด addressable market สำหรับ production graph ขนาดใหญ่

### ⚠️ hop6 latency ใกล้ DuckDB (4.40 vs 4.67 ms)

DuckDB set-based recursive join เริ่ม catch up ที่ depth สูง เป็น signal ว่า deep
traversal optimization อาจจำเป็นถ้า use case ต้องการ hop5+

---

## Bright Spots

1. **LadybugDB 168× hop1** — คู่แข่ง on-niche ที่สุด (embedded, graph+vec, regulated
   industries) แพ้ชัดเจนทุก depth และ GenesisBlock ยัง materialize full node+path ไม่ใช่
   bare id — ตัวเลขนี้ใช้ marketing ได้ทันที

2. **Governance overhead <0.1%** — MASTER tier ที่ไม่มีใครในตลาดทำ แต่ cost แทบ zero
   (~524 ns/op) พิสูจน์แล้ว ใช้เป็น selling point ได้โดยไม่ต้องกลัวว่าจะโดน dismiss
   ว่าช้า

3. **8/8 competitor วัดครบ** — engineering milestone สำคัญ ไม่มีตัวเลขที่ "อ้างโดยไม่มี
   หลักฐาน" อีกต่อไป เปิด era ของ evidence-based competitive positioning

4. **O(neighborhood) traversal พิสูจน์แล้ว** — hop1 อยู่ที่ 21–35 µs ข้าม 100× scale
   (10k→1M nodes) แสดงว่า adjacency index architecture ถูกต้อง

5. **Recall–latency frontier tunable** — ef_search เป็น live knob ทำให้ user เลือก
   trade-off ได้ ไม่ถูก lock ไว้ที่จุดเดียว

---

## Areas of Concern

### 🔴 Priority 1: RAM ceiling ที่ 1M nodes

- **ปัญหา**: Edge UUID interning กิน ~11× RAM มากกว่า Kuzu — 1M/8M graph ใช้ 12.6 GB
- **Root cause**: ทุก edge ID ถูก intern เป็น string ใน DashMap,
  Kuzu ใช้ columnar storage ไม่มี string interning
- **Impact**: จำกัด addressable market สำหรับ production-scale knowledge graph
- **Action ที่ต้องทำ**: วัด RAM impact ของ edge interning alternatives ก่อนเลือก fix

### ⚠️ Priority 2: Recall@10 ตกที่ 500k+ vectors

- **ปัญหา**: Recall ตกจาก 0.982 (100k) → 0.891 (500k) ที่ ef_search=100
- **Action ที่ต้องทำ**: Test ef_search=200 ที่ 500k เพื่อ confirm ว่า recall กลับมาได้
- **Hypothesis**: ef_search=100 ไม่เพียงพอที่ scale สูง — เป็น tuning issue ไม่ใช่ index bug

### ⚠️ Priority 3: Ingest throughput vs columnar systems

- GenesisBlock durable 1,751 vec/s vs Kuzu/LadybugDB ~60× เร็วกว่า (COPY)
- ไม่ใช่ bug แต่เป็น architectural trade-off ที่ต้องมี messaging ชัดเจนว่า
  "GenesisBlock for low-latency agent memory, Kuzu for bulk graph analytics"
- **Action ที่ต้องทำ**: เพิ่ม use case guidance ใน README และ docs

---

## Recommended Actions

| # | Action | Priority | เหตุผล |
|---|---|---|---|
| 1 | วัด RAM impact ของ edge interning alternatives | 🔴 สูงมาก | ปลดล็อค >1M node market |
| 2 | Test recall@10 ที่ 500k ด้วย ef_search=200 | 🟡 กลาง | ปิด "recall ตกที่ scale" concern |
| 3 | เผยแพร่ benchmark page สาธารณะจาก P15–P30 | 🟡 กลาง | ตัวเลขมีแล้ว อย่าปล่อยทิ้ง — LadybugDB/HelixDB อาจ claim ก่อน |
| 4 | TypeScript type defs + "install in 5 min" example | 🟡 กลาง | NAPI addon คือ moat ที่สร้างใหม่ยาก แต่ต้องมี DX ที่ดีจึงจะ unlock ได้ |
| 5 | วัด hop6 deep traversal optimization options | 🟢 ต่ำ | DuckDB tied ที่ 4.67 ms แต่ยังนำอยู่ — ยังไม่เร่งด่วน |

---

## Context & Caveats

- ตัวเลขทั้งหมดจาก session เดียว (2026-06-21) บนเครื่องเดียว — ยังไม่มี external
  validation หรือ CI benchmark pipeline
- **Disk matters**: G: (7200 RPM HDD) slow กว่า C: (SSD) 42–46× สำหรับ fsync-bound
  writes — insert numbers เปรียบกับ competitor บน SSD ต้องใส่ caveat
- **Durability asymmetry**: GenesisBlock persist ทุก write; Chroma test รันแบบ in-memory
  — insert comparison ไม่ apples-to-apples, query latency และ recall เป็น fair metric
- **P30 LadybugDB payload note**: LadybugDB Cypher return bare id, GenesisBlock
  materialize full node+path object — GenesisBlock ทำงานมากกว่าแต่ยังชนะ 168× บน hop1
- **RAM benchmark caveat**: 15.89 GB figure เก่า (P12) เป็น artifact ของ Mark VII
  build — ตัวเลขปัจจุบัน (~82 MB @5k, ~1.06 GB @100k) มาจาก measurement โดยตรง

---

## Metrics ที่ควรมีแต่ยังไม่มี

| Metric | เหตุผลที่ควรติดตาม |
|---|---|
| Real-world graph recall (multi-hop semantic) | ยังใช้ synthetic clustered vectors อยู่ |
| Concurrent read/write benchmark | รู้แค่ concurrent write — ยังไม่วัด mixed workload |
| Memory @ 500k vectors | รู้แค่ 100k (1.57 GB) และ 500k scale แต่ recall ตก |
| CI benchmark pipeline | ปัจจุบัน run manually — ไม่มี regression detection อัตโนมัติ |
| P2P sync latency | CRDT sync เป็น feature หลัก แต่ยังไม่มี benchmark |

---

*Review ถัดไป: สัปดาห์สิ้นสุด 29 มิถุนายน 2026*
*ข้อมูล: เพิ่ม P31 (edge interning alternatives) และ P32 (recall@500k, ef_search sweep)*
