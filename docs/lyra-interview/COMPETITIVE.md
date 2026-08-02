# COMPETITIVE - LYRA

## Scope

[derived] "Tier นี้" หมายถึง local-first / embedded / agent-memory engine tier: ระบบที่ buyer ใช้เป็นหน่วยความจำ persistent สำหรับ agent/orchestrator/on-device agent โดยต้องมีอย่างน้อยหนึ่งแกนของ vector, graph, temporal/bitemporal, sync, หรือ fusion ในเส้นทาง hot path. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:209] [docs/ROUND3-LOCAL-MODEL-FINDINGS.md:46]

[derived] ไม่ควรนิยามคู่แข่งเป็น "database ทุกตัว" เพราะหลักฐานใน repo แยก Neo4j/Qdrant เป็น reference ที่รู้จักดี แต่ไม่ใช่ category หลักของ embedded graph+vector agent-memory engine. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:209]

[derived] คำตอบสั้น: คู่แข่งหลักใน tier นี้คือ (1) SQLite/libSQL stack, (2) LadybugDB, (3) HelixDB, (4) DuckDB/RocksDB style embedded baselines, และ (5) compose-at-app stack เช่น Qdrant + Kuzu/Neo4j + app-layer RRF/temporal filter. [docs/genesis-interview/evidence/r2-sqlite.md:60] [docs/genesis-interview/evidence/r2-competitors.md:53] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56]

## Ranked Competitors

### 1. SQLite / libSQL + sqlite-vec + recursive CTE + trigger history

**Threat level:** HIGH.

[derived] นี่คือคู่แข่งที่อันตรายที่สุดเชิง adoption เพราะ buyer ไม่ต้อง "ซื้อ thesis" ใหม่: ใช้ SQLite WAL, sqlite-vec/libSQL vector, recursive CTE, และ trigger/history tables ได้ในไฟล์เดียว. [docs/genesis-interview/evidence/r2-sqlite.md:55] [docs/genesis-interview/evidence/r2-sqlite.md:60]

[asserted] หลักฐานท้องถิ่นบอกว่า SQLite WAL ให้ snapshot consistency จริง และ all-in-one SQLite assembly ไม่ควรถูกโจมตีด้วย claim ว่า cross-store consistency แตกง่าย. [docs/genesis-interview/evidence/r2-sqlite.md:55] [docs/genesis-interview/evidence/r2-sqlite.md:57]

[derived] จุดที่ GenesisBlockDB ยังโจมตี SQLite ได้อย่างซื่อสัตย์คือ scale/latency ของ combined workload และ bitemporal correctness ที่ built-in มากกว่า consistency. [docs/genesis-interview/evidence/r2-sqlite.md:57] [docs/genesis-interview/evidence/r2-sqlite.md:60]

[asserted] sqlite-vec stable line ยังเป็น exact scan; ANN อยู่ใน alpha และยังมี maturity caveats. [docs/genesis-interview/evidence/r2-sqlite.md:5] [docs/genesis-interview/evidence/r2-sqlite.md:9]

[asserted] raw SQLite ไม่มี native system-versioning/bitemporal; ต้องใช้ triggers/history tables และมีปัญหา stable transaction time สำหรับ temporal-table equivalence. [docs/genesis-interview/evidence/r2-sqlite.md:42] [docs/genesis-interview/evidence/r2-sqlite.md:50]

**Falsifier:** [derived] ถ้า SQLite/libSQL stack ทำ Q1-Q3 ของ G3 ได้ใน p50/p99 ใกล้ GenesisBlockDB, round trip ไม่แพ้, และ trigger-history พอสำหรับ audit จริง คู่แข่งนี้ชนะด้วย simplicity. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:84]

### 2. LadybugDB

**Threat level:** HIGH.

[asserted] LadybugDB เป็น community successor ของ Kuzu และมี release ล่าสุด v0.18.0 ในหลักฐาน local; extension docs ระบุ vector search, FTS, JSON, graph algorithms และอื่น ๆ. [docs/genesis-interview/evidence/r2-competitors.md:14] [docs/genesis-interview/evidence/r2-competitors.md:15]

[measured] รายงาน P30 บอกว่า LadybugDB เป็น competitor ที่ on-niche มากที่สุด: embedded graph+vector, Kuzu fork, วัดกับ 100k/800k แล้ว GenesisBlockDB ชนะ traversal latency แต่ LadybugDB ชนะ ingest ประมาณ 48x และ memory ประมาณ 11x. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:253]

[derived] LadybugDB เป็นคู่แข่งตรงกว่า Chroma/Qdrant เพราะมี graph engine อยู่ใน embedded tier และขาดหลัก ๆ ที่ bitemporal/governance/signed events. [docs/genesis-interview/evidence/r2-competitors.md:56] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:91] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:92]

[unknown] P30 วัด LadybugDB v0.15.3 แต่ ADR ระบุว่า v0.18.0 มี WAL group commits และ ART indexes; ยังไม่มี P35 re-run. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:244]

**Falsifier:** [derived] ถ้า LadybugDB v0.18 เพิ่ม temporal/audit ที่พอใช้ หรือชนะ GenesisBlockDB ใน mixed graph+vector+time workload หลัง re-run, "เราเป็น engine ที่ชัดกว่า" จะอ่อนลงมาก. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199]

### 3. HelixDB

**Threat level:** HIGH-UNKNOWN.

[asserted] ADR ระบุ HelixDB เป็น closest live same-tier Rust rival และยังไม่มี direct GenesisBlockDB-vs-HelixDB benchmark; vendor bench ใช้ topology/hardware คนละแบบ. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:111]

[derived] เพราะยังไม่วัด head-to-head เลย HelixDB ไม่ควรถูกจัดว่าแพ้หรือชนะ GenesisBlockDB; ต้องจัดเป็น measurement gap อันดับหนึ่ง. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:244]

[asserted] ADR บอกว่า "sole custom QL in this category" เป็น false เพราะ HelixDB มี HelixQL, LadybugDB มี Cypher, และ Neo4j Cypher 25 มี vector SEARCH. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:223]

**Falsifier:** [derived] ถ้า P34 แสดงว่า HelixDB ชนะ graph/vector/fusion workload หรือเพิ่ม temporal+governance ก่อน GenesisBlockDB ปิด ops floor, HelixDB จะกลายเป็น direct #1. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:178] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199]

### 4. Compose-at-app stack: Qdrant + Kuzu/Neo4j + app-layer RRF/temporal filter

**Threat level:** HIGH for orchestrator buyer.

[derived] สำหรับ buyer ฝั่ง orchestrator คู่แข่งจริงอาจไม่ใช่ database เดี่ยว แต่เป็น stack ที่ประกอบ vector store + graph store + application fusion เอง. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56]

[asserted] BENCH-SPEC เรียก B1 ว่า "the real competitor": Qdrant สำหรับ vector, Kuzu/Neo4j สำหรับ graph, และ TS/Python layer ทำ RRF fusion กับ AS OF filtering โดยนับทุก round trip/serialization/glue latency. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:58]

[derived] นี่คือคู่แข่งที่ falsify G3 ได้ตรงที่สุด: ถ้า app-side fusion พอเร็วและ debug ง่ายกว่า HQL/G3, buyer จะไม่ย้ายเข้า engine. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:84] [docs/lyra-interview/ROUND3.md:8]

**Falsifier:** [derived] G3 moat ควรถูก kill ถ้า HQL saving ต่ำกว่า 20% p50 และไม่ลด round trips อย่างน้อย 2x; proceed ได้เฉพาะถ้าลด round trips อย่างน้อย 2x และประหยัด p50 อย่างน้อย 30% บน Q1-Q3. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:84] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:87]

### 5. DuckDB + graph/CTE and RocksDB + hand-rolled graph

**Threat level:** MEDIUM-HIGH.

[measured] DuckDB+graph เป็น embedded baseline ที่ปิด gap ได้ลึก: GenesisBlockDB ชนะ hop1 ประมาณ 54x แต่ hop6 เหลือประมาณ 1.06x หรือ effectively tied. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:236] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:151]

[measured] RocksDB+graph เป็น architecturally-close baseline; hop1 tied-class แต่ RocksDB ชนะ ingest ประมาณ 30x และ memory ประมาณ 32x โดยแลกกับการไม่มี query language, paths, governance, bitemporal, หรือ vectors ในตัว. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:243]

[derived] คู่แข่งกลุ่มนี้ไม่ได้ชนะ narrative product เต็มตัว แต่ชนะด้าน "ทำเองแบบ lean และถูกกว่า" โดยเฉพาะถ้า buyer ต้องการแค่ adjacency + lookup. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:243] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:36]

**Falsifier:** [derived] ถ้า GenesisBlockDB ไม่ปิด RAM/bulk-ingest/deep-hop gaps, embedded baselines จะฆ่า same-tier superiority แม้ GenesisBlockDB มี semantics ดีกว่า. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:36] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:209]

### 6. LanceDB, Chroma, Qdrant, Neo4j, SurrealDB

**Threat level:** MEDIUM as references; HIGH only when bundled into a stack.

[measured] Chroma, Qdrant, LanceDB, Neo4j, Kuzu, DuckDB+graph, RocksDB+graph, และ LadybugDB มี measured head-to-head ใน report. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:214]

[measured] LanceDB เป็น embedded vector competitor ที่ GenesisBlockDB ชนะ point-query p50 ใน P27 แต่ caveat คือวัดที่ recall ต่ำกว่า LanceDB และ LanceDB มี larger-than-memory/on-disk vector story ที่ GenesisBlockDB ยังไม่มี. [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md:230] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:151]

[measured] Chroma คือ fair peer สำหรับ G1 vector มากกว่า Qdrant server; BENCH-SPEC สรุปว่า G1 เป็น embedded parity ไม่ใช่ dominance. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:17]

[measured] Qdrant server number มี network/server tax และ GenesisBlockDB recall ceiling ยังต่ำกว่า Qdrant ในบาง condition; จึงไม่ควร claim vector dominance แบบกว้าง. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:17] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:36]

[asserted] Neo4j hybrid-search threat materialized ด้วย Cypher 25 SEARCH GA; เป็น enterprise/reference threat มากกว่า embedded-tier peer. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:35]

[asserted] SurrealDB CRDT nightmare ไม่เกิดขึ้นตาม ADR; แต่ enterprise floor ยังละเมิด 10/16 axes ทำให้ GenesisBlockDB ยังต่ำกว่า enterprise bar ด้าน ops. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:35] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:141]

## Competitive Position

[derived] ตำแหน่งที่ defensible ไม่ใช่ "ไม่มีคู่แข่ง" แต่คือ: "ในหลักฐานปัจจุบัน ยังไม่พบ single in-process embedded engine ที่รวม graph traversal + vector ANN + engine-enforced row-level bitemporal + signed/verifiable governance ใน binary เดียว." [docs/genesis-interview/evidence/r2-competitors.md:56] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:91] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:92]

[derived] ตำแหน่งนี้ยังขายไม่ได้ถ้าไม่ปิด gaps: REST global RwLock/no proven MVCC snapshot, mobile FFI ยังไม่มี sync, phone-cloud CRDT ยังไม่พิสูจน์, enterprise ops floor ต่ำกว่า 10/16 axes. [docs/lyra-interview/ROUND3.md:21] [docs/lyra-interview/ROUND3.md:30] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:141]

[derived] ถ้าจะชนะใน tier นี้ ต้องชนะ "good enough" ก่อนชนะ vendor: SQLite/libSQL คือ null hypothesis, Ladybug/Helix คือ direct engine threats, และ compose-at-app stack คือ orchestrator alternative. [docs/genesis-interview/evidence/r2-sqlite.md:60] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56]

## What To Measure Next

1. [derived] P34: HelixDB head-to-head, same harness and dataset as same-tier graph/vector workload. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199]
2. [derived] P35: LadybugDB v0.18 re-run, because current P30 is stale against v0.18. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:244]
3. [derived] SQLite/libSQL null-hypothesis harness: sqlite-vec/libSQL vector + recursive CTE + trigger history + WAL snapshot, scored on Q1-Q3. [docs/genesis-interview/evidence/r2-sqlite.md:60] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:65]
4. [derived] G3 B1 harness: Qdrant + Kuzu/Neo4j + app-layer RRF/temporal filter vs single HQL query, with p50/p99, round trips, bytes-over-wire, RAM, and CI. [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:76]
5. [derived] Concurrency/snapshot harness: multi-agent/multi-process writes, reads, compaction, and historical queries to settle whether GenesisBlockDB's bitemporal semantics satisfy orchestrator concurrency needs. [docs/lyra-interview/ROUND3.md:21]
6. [derived] Mobile CRDT trial: phone offline edits + cloud/orchestrator concurrent edits + reconnect via mobile-safe transport, because current CRDT asset is core-tested but not phone-cloud-proven. [docs/lyra-interview/ROUND3.md:30]

## OPEN-QUESTIONS

- [unknown] Is HelixDB currently faster/slower than GenesisBlockDB on the same 100k/800k fixture and same materialization contract? [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52]
- [unknown] Does LadybugDB v0.18 close enough of the ingest/WAL/index gap to change the P30 verdict? [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:199]
- [unknown] Can libSQL DiskANN + recursive CTE + trigger history satisfy the orchestrator's real Q1-Q3 jobs inside one consistent SQLite transaction? [docs/genesis-interview/evidence/r2-sqlite.md:55] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:65]
- [unknown] Do orchestrator buyers prefer engine-side HQL/G3, or do they keep fusion in the app for debuggability and cache control? [docs/lyra-interview/ROUND3.md:8]
- [unknown] What exact buyer or product has committed to graph+vector+bitemporal instead of SQLite/Redis/app-fusion? [docs/lyra-interview/ROUND3.md:19]

## LYRA Verdict

[derived] ใน tier นี้ คู่แข่งตัวจริงอันดับหนึ่งไม่ใช่ cloud vector DB เดี่ยว ๆ แต่คือ "good enough local stack" โดยเฉพาะ SQLite/libSQL; direct engine threats คือ LadybugDB และ HelixDB; และ orchestrator threat คือ compose-at-app stack. [docs/genesis-interview/evidence/r2-sqlite.md:60] [docs/genesis-interview/evidence/r2-competitors.md:53] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:52] [docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md:56] [derived] GenesisBlockDB มี wedge ที่ชัดใน bitemporal + signed/verifiable governance + hybrid engine semantics แต่ยังห้าม oversell จนกว่าจะวัด Helix/Ladybug ใหม่, ฆ่าหรือยืนยัน SQLite null hypothesis, และพิสูจน์ G3/mobile/concurrency ด้วย harness จริง. [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:91] [docs/ADR--GENESISDB-COMPETITIVE-SUPERIORITY.md:92] [docs/lyra-interview/ROUND3.md:39]
