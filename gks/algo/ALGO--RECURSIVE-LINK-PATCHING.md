---
id: ALGO--RECURSIVE-LINK-PATCHING
phase: 2
type: algo
status: active
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: Recursive Link Patching — Regex-based Transformation
created_at: 2026-05-28T16:05:00.000+07:00
tags: [algo, regex, automation]
aliases:
  - ALGO
  - implementation_flow
  - Computational logic / Step-by-step
cluster: implementation_flow
role: Computational logic / Step-by-step
crosslinks:
  references:
    - BLUEPRINT--GKS-REORG-RUNBOOK
linked_symbols:
  - { file: "scripts/msp/re-indexer.ts" }
attributes:
  domain: logic
---

# ALGO — Recursive Link Patching

## Steps

1. **Find:** ค้นหารูปแบบ `\[.*?\]\((?!http)(\.\.\/|.*\.md)\)` ในไฟล์ Markdown
2. **Extract:** ดึงชื่อไฟล์เป้าหมายออกมา
3. **Lookup:** ค้นหา ID ของไฟล์นั้นจาก `atomic_index.jsonl`
4. **Replace:** แทนที่ Path ด้วย `[[ID]]`
