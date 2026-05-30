---
id: GENESIS--BACKEND-ENGINE
phase: 1
type: genesis
status: stable
vault_id: GKS-CORE
tier: genesis
source_type: axiomatic
title: "Genesis: GenesisDB Graph Engine Root Orchestrator"
tags: [gks, genesis, backend, graph-db, architecture-root]
aliases: [genesis-db-root]
crosslinks:
  orchestrates:
    - ADR--GENESISDB-CSR-MUTATION-STRATEGY
    - ADR--GENESISDB-GOVERNANCE-LOGIC
    - ADR--GENESISDB-TEMPORAL-MODEL
    - ADR--GENESISDB-KIMPACT-ALGORITHM
    - ADR--GENESISDB-BENCHMARK-SUITE
    - ADR--GENESISDB-SCALABILITY-VALIDATION
    - ALGO--KIMPACT-CALCULATION
    - SPEC--K-IMPACT
    - AUDIT--GENESIS-DB-LDBC-LITE-REPORT
attributes:
  engine_v: 1.x
  runtime: rust-native
---

# GENESIS--BACKEND-ENGINE

## 1. Executive Mission
`GENESIS--BACKEND-ENGINE` ทำหน้าที่เป็น "จุดกำเนิด" และศูนย์กลางการควบคุมโครงสร้างสถาปัตยกรรมของ **GenesisDB** (Embedded Rust Graph Engine) อะตอมนี้ทำหน้าที่เชื่อมโยงการตัดสินใจเชิงเทคนิค (ADRs) อัลกอริทึม (ALGOs) และมาตรฐานประสิทธิภาพ (AUDIT) เข้าด้วยกันเพื่อความสะดวกในการบำรุงรักษาและการสืบค้นความรู้

## 2. Architecture Decision Records (Engineering Proofs)
รายการตัดสินใจเชิงลึกที่ประกอบกันขึ้นเป็นเครื่องยนต์ฐานข้อมูล:

*   **Storage Strategy:** [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]] — การจัดการหน่วยความจำแบบ Chunked-CSR และ Slack Space เพื่อ High-Throughput Write.
*   **Logic & Safety:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]] — ระบบ Axiomatic Governance และ Transitive Contradiction Checking.
*   **Temporal Model:** [[ADR--GENESISDB-TEMPORAL-MODEL]] — การทำ Bi-Temporal Graph สมบูรณ์แบบผ่าน Value-History Arenas.
*   **Ranking Strategy:** [[ADR--GENESISDB-KIMPACT-ALGORITHM]] — เหตุผลเชิงกลยุทธ์ในการเลือกใช้ K-Impact แทน PageRank.
*   **Verification Standards:** [[ADR--GENESISDB-BENCHMARK-SUITE]] — ระเบียบวิธีวิจัยและการทำ Reproducible Benchmarks.
*   **Scale Assurance:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]] — การพิสูจน์ทางวิศวกรรมเรื่องการรองรับ 500M Edges.

## 3. Core Logic & Implementation
*   **Scoring Algorithm:** [[ALGO--KIMPACT-CALCULATION]] — สูตรคณิตศาสตร์และ Pseudo-code การคำนวณ K-Impact.
*   **Base Specification:** [[SPEC--K-IMPACT]] — นิยามมิติ DD, AS, SC ในเชิงโครงสร้าง.

## 4. Quality & Performance Reports
*   **LDBC-Lite Audit:** [[AUDIT--GENESIS-DB-LDBC-LITE-REPORT]] — รายงานผล Benchmark เปรียบเทียบกับ Neo4j และ Memgraph.
*   **Recovery Audit:** [[AUDIT--GENESIS-BACKEND-RECOVERY-REFINEMENT]] — บันทึกผลการกู้คืนระบบจากสถาพ Degraded สู่ Healthy.

## 5. Non-Atomic Documentation (External Assets)
เอกสารมาตรฐานวิศวกรรมสากลที่จัดเก็บใน `docs/gks/genesis-db/`:
*   `WHITEPAPER--GENESIS-DB.md`: เอกสารนำเสนอแนวคิดระดับสูงและปรัชญาของระบบ.
*   `SPEC--GENESIS-DB.md`: ข้อกำหนดทางเทคนิค (API & Internal Subsystems).
*   `EXPANSION-SPEC--GENESIS-DB.md`: รายละเอียดเชิงลึกระดับ Systems Engineering.

## 6. Maintenance Policy
การแก้ไขสถาปัตยกรรมระดับลึกของ GenesisDB ต้องอ้างอิงและอัปเดตอะตอมในลู่นี้ (Chain) เสมอ โดยต้องผ่านการทำ `npm run msp:validate` เพื่อรักษาความสมบูรณ์ของ Knowledge Graph
