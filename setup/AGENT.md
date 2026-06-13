# GoVibe Universal Agent Contract

## 1. Mission
Agent ทุกตัวมีหน้าที่รักษาความถูกต้องของ Knowledge Flow (GKS) และปฏิบัติตามระเบียบการ Execution Governance อย่างเคร่งครัด

## 2. Core Principles
- **R1 - Think before action:** ไม่สมมติฐานเด็ดขาด หากไม่แน่ใจให้สอบถามเสมอ
- **R2 - Simplicity first:** เขียนเฉพาะสิ่งที่ถูกสั่ง ไม่มีฟีเจอร์แฝง
- **R3 - Surgical changes:** แก้ไขเฉพาะจุดที่ได้รับมอบหมาย รักษา Coding Style เดิมไว้
- **R4 - Goal-driven execution:** ต้องกำหนด Success Criteria (Test/Doc) ก่อนลงมือเสมอ

## 3. Communication Protocol (Handoff)
- ใช้ `.agents/devops/handoff/log.jsonl` ในการส่งต่องานเสมอ
- งานที่ยังไม่เสร็จ (Revoke) ต้องระบุ Blocker ไว้ใน log ชัดเจน

## 4. Execution Governance (H0-H6)
- **Atomic (H0-H1):** Single-file edit, ต้องผ่าน Validator ก่อน Commit
- **System-Wide:** การแก้ไขข้ามโมดูลต้องมี ADR อ้างอิงเสมอ
