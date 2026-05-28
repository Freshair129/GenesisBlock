---
id: PARAMS--GKS-REORG-THRESHOLDS
phase: 2
type: params
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: GKS Reorganization Thresholds and Config
created_at: 2026-05-28T16:15:00.000+07:00
tags: [params, config, constants]
aliases:
  - PARAMS
  - implementation_flow
  - Parameters / Constants / Config
cluster: implementation_flow
role: Parameters / Constants / Config
crosslinks:
  references:
    - BLUEPRINT--GKS-REORG-RUNBOOK
linked_symbols: []
attributes:
  domain: general
---

# PARAMS — GKS Reorg Thresholds

| Param | Value | Purpose |
| :--- | :--- | :--- |
| `MAX_BATCH_SIZE` | 50 | จำนวนไฟล์สูงสุดที่จัดการใน 1 Turn |
| `STRICT_ID_MATCH` | true | บังคับให้ ID ต้องตรงกับ Filename 100% |
| `REORG_TARGET_VERSION` | 2.3.0 | มาตรฐาน Taxonomy เป้าหมาย |
