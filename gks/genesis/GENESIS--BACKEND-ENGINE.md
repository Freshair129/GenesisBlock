---
id: GENESIS--BACKEND-ENGINE
phase: 0
type: genesis
status: active
vault_id: GKS-CORE
tier: master
source_type: axiomatic
title: "Genesis: GenesisDB Graph Engine Root Orchestrator"
tags: [gks, genesis, backend, graph-db, architecture-root]
aliases: [genesis-db-root]
created_at: 2026-05-30T04:00:00+07:00
promoted_from: genesis
promoted_at: 2026-05-30T04:00:00+07:00
promotion_adr: ADR--MASTER-PROMOTION-DOC-TO-CODE
manifest_version: "1.0"
daci:
  driver: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
members:
  core:
    cognitive: [COGNITIVE--SHARED-BRAIN-PHILOSOPHY]
    algo: [ALGO--KIMPACT-CALCULATION]
    concept: [CONCEPT--GENESIS-GRAPH-BACKEND]
    runbook: [RUNBOOK--BRAIN-SYNC-PROCEDURE]
    params: [PARAMS--GKS-REORG-THRESHOLDS]
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

## Executive Mission
`GENESIS--BACKEND-ENGINE` ทำหน้าที่เป็น "จุดกำเนิด" และศูนย์กลางการควบคุมโครงสร้างสถาปัตยกรรมของ **GenesisDB** (Embedded Rust Graph Engine) อะตอมนี้ทำหน้าที่เชื่อมโยงการตัดสินใจเชิงเทคนิค (ADRs) อัลกอริทึม (ALGOs) และมาตรฐานประสิทธิภาพ (AUDIT) เข้าด้วยกันเพื่อความสะดวกในการบำรุงรักษาและการสืบค้นความรู้

## Architecture Decision Records (Engineering Proofs)
รายการตัดสินใจเชิงลึกที่ประกอบกันขึ้นเป็นเครื่องยนต์ฐานข้อมูล:

*   **Storage Strategy:** [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]] — การจัดการหน่วยความจำแบบ Chunked-CSR และ Slack Space เพื่อ High-Throughput Write.
*   **Logic & Safety:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]] — ระบบ Axiomatic Governance และ Transitive Contradiction Checking.
*   **Temporal Model:** [[ADR--GENESISDB-TEMPORAL-MODEL]] — การทำ Bi-Temporal Graph สมบูรณ์แบบผ่าน Value-History Arenas.
*   **Ranking Strategy:** [[ADR--GENESISDB-KIMPACT-ALGORITHM]] — เหตุผลเชิงกลยุทธ์ในการเลือกใช้ K-Impact แทน PageRank.
*   **Verification Standards:** [[ADR--GENESISDB-BENCHMARK-SUITE]] — ระเบียบวิธีวิจัยและการทำ Reproducible Benchmarks.
*   **Scale Assurance:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]] — การพิสูจน์ทางวิศวกรรมเรื่องการรองรับ 500M Edges.

## Core Logic & Implementation
*   **Scoring Algorithm:** [[ALGO--KIMPACT-CALCULATION]] — สูตรคณิตศาสตร์และ Pseudo-code การคำนวณ K-Impact.
*   **Base Specification:** [[SPEC--K-IMPACT]] — นิยามมิติ DD, AS, SC ในเชิงโครงสร้าง.

## Quality & Performance Reports
*   **LDBC-Lite Audit:** [[AUDIT--GENESIS-DB-LDBC-LITE-REPORT]] — รายงานผล Benchmark เปรียบเทียบกับ Neo4j และ Memgraph.
*   **Recovery Audit:** [[AUDIT--GENESIS-BACKEND-RECOVERY-REFINEMENT]] — บันทึกผลการกู้คืนระบบจากสถาพ Degraded สู่ Healthy.

## Non-Atomic Documentation (External Assets)
เอกสารมาตรฐานวิศวกรรมสากลที่จัดเก็บใน `docs/gks/genesis-db/`:
*   `WHITEPAPER--GENESIS-DB.md`: เอกสารนำเสนอแนวคิดระดับสูงและปรัชญาของระบบ.
*   `SPEC--GENESIS-DB.md`: ข้อกำหนดทางเทคนิค (API & Internal Subsystems).
*   `EXPANSION-SPEC--GENESIS-DB.md`: รายละเอียดเชิงลึกระดับ Systems Engineering.

## Maintenance Policy
การแก้ไขสถาปัตยกรรมระดับลึกของ GenesisDB ต้องอ้างอิงและอัปเดตอะตอมในลู่นี้ (Chain) เสมอ โดยต้องผ่านการทำ `npm run msp:validate` เพื่อรักษาความสมบูรณ์ของ Knowledge Graph
