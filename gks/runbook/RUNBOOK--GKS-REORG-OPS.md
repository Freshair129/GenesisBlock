---
id: RUNBOOK--GKS-REORG-OPS
phase: 6
type: runbook
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: GKS Reorganization Operational Response Guide
created_at: 2026-05-28T16:10:00.000+07:00
tags: [runbook, ops, incident-response]
aliases:
  - RUNBOOK
  - implementation_flow
  - Operational response guide
cluster: implementation_flow
role: Operational response guide
crosslinks:
  references:
    - BLUEPRINT--GKS-REORG-RUNBOOK
linked_symbols: []
attributes:
  domain: general
---

# RUNBOOK — GKS Reorg Ops

## Trigger

ใช้เมื่อมีการย้ายโฟลเดอร์ใน `gks/` หรือตรวจพบ Broken Links จำนวนมาก

## Steps

1. หยุดการเขียน Atom ใหม่ชั่วคราว
2. รัน `npm run msp:index` เพื่อซ่อมแซมดัชนี
3. หาก Graph พัง ให้ใช้ Rollback Map ที่สร้างไว้ก่อนรันสคริปต์
