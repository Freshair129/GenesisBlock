---
id: PROTO--BRAIN-INDEXING-MANDATE
phase: 2
type: proto
status: stable
vault_id: default
tier: safety
source_type: axiomatic
title: Brain Indexing Mandate
tags:
  - msp
  - gks
  - proto
  - sync
crosslinks:
  references:
    - ADR--SHADOW-REPO-SHARED-BRAIN
    - RUNBOOK--BRAIN-SYNC-PROCEDURE
created_at: 2026-05-29T12:05:00.000+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  severity: error
  domain: proto
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PROTO — Brain Indexing Mandate

## Rule

ทุกๆ Local Environment และขั้นตอนการทำงาน (Development Cycle) ที่มีการอัปเดต เพิ่มเติม หรือลบไฟล์ Markdown ในโฟลเดอร์ `gks/` จะต้องทำการรันคำสั่ง `npm run msp:index` เพื่อจำลองดัชนีโครงสร้างความรู้ `gks/00_index/atomic_index.jsonl` ใหม่เสมอ เพื่อคงความสอดคล้องของข้อมูลและการันตีกระบวนการตรวจสอบก่อนนำขึ้น Git Remote (Shadow Brain Repo)

## Severity

error

## Enforcement

ระบบตรวจสอบความถูกต้อง (Validator Pipeline) จะตรวจสอบลิงก์อ้างอิงและจุดเชื่อมต่อสัญลักษณ์ของทุกอะตอม หากมีดัชนีเก่าหรือเกิดจุดอ้างอิงปลายทางสูญหาย (Dangling Wikilinks) จะส่งผลให้ระบบ Validation ล้มเหลวและบล็อกการ Deployment หรือ Commit

## Counter-example

การ Push ไฟล์อะตอมใหม่ `CONCEPT--NEW-FEATURE.md` เข้าสู่ Remote Branch โดยไม่ผ่านการตรวจสอบและไม่รันดัชนีใหม่ ซึ่งอาจส่งผลให้เกิด Broken Links ในอะตอมต้นทางที่มีจุดเชื่อมโยงถึงอะตอมดังกล่าว

## Source

- Phase 0 doc-to-code requirement.
