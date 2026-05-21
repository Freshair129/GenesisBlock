# INCIDENT REPORT: Antigravity Agent Failure (Git Redundancy Conflict)

**Date:** 2026-05-13
**Status:** Resolved (Pending Client Restart)
**Severity:** High (Agent Functionality Blocked)

## 1. Symptoms (อาการ)

- Antigravity Agent ไม่ทำงาน (Fail to start/respond).
- เครื่องมือ (Tools) ใน Antigravity Cockpit ไม่แสดงผลหรือเรียกใช้งานไม่ได้.
- ระบบ Git ใน IDE แสดงสถานะ Error หรือไม่สามารถหา Root ของโปรเจกต์เจอ.
- ใน Log (`Git.log`) พบข้อความ `Failed to parse HEAD file: ENOENT` ชี้ไปยังเส้นทางที่ไม่มีอยู่จริง.

## 2. Root Cause (ที่มาของปัญหา)

ปัญหานี้เกิดจาก **"Git Redundancy (ระบบ Git ซ้ำซ้อน)"** โดยมีรายละเอียดดังนี้:

- **Nested Worktree:** มีการใช้ Claude Code ในโปรเจกต์เดียวกัน ซึ่งสร้าง Git Worktree ไว้ในโฟลเดอร์ `.claude/worktrees/`.
- **Conflict:** โฟลเดอร์ Worktree นี้มีไฟล์ `.git` ซ่อนอยู่ เมื่อ Antigravity Agent เริ่มต้นทำงาน มันจะทำการสแกนหา Git Root แต่ดันไปพบไฟล์ `.git` ของ Claude ก่อน ทำให้มันเข้าใจผิดว่า Root ของโปรเจกต์อยู่ที่โฟลเดอร์ชั่วคราวนั้น.
- **Dead State:** เมื่อเอเจนท์พยายามอ่านไฟล์ผ่าน Git ใน Root ที่ผิดพลาด ทำให้เกิด Exception และตัว Extension Host ของ Antigravity หยุดทำงาน (Crash/Terminate).

## 3. Resolution (การแก้ไข)

ผมได้ดำเนินการแก้ไขในส่วนของโครงสร้างไฟล์และระบบ Git ดังนี้:

1. **Worktree Removal:** ลบ Git Worktree ที่ค้างอยู่ใน `.claude/` ออกทั้งหมดด้วยคำสั่ง `git worktree remove --force`.
2. **Git Branch Cleanup:** ลบ Local Branch ที่เกี่ยวข้องกับ Worktree ที่พัง เพื่อป้องกันการอ้างอิงกลับมาอีก.
3. **Git Maintenance:** รัน `git prune` และ `git gc` เพื่อล้าง Unreachable Objects และปรับแต่งฐานข้อมูล Git ให้สมบูรณ์.
4. **Exclusion Rules:** แก้ไขไฟล์ `.gitignore` โดยเพิ่มกฎ `.claude/` เพื่อป้องกันไม่ให้ Antigravity หรือเอเจนท์อื่นๆ สแกนเข้าไปเจอระบบ Git ภายในโฟลเดอร์ของ Claude ในอนาคต.
5. **Index Regeneration:** รัน `npm run msp:index` เพื่อสร้าง Atomic Index ใหม่ให้ระบบจำโครงสร้างไฟล์ที่ถูกต้อง.

## 4. Verification (การยืนยัน)

- ตรวจสอบผ่านคำสั่ง `Get-ChildItem -Recurse -Filter .git` ยืนยันว่าเหลือเพียงไฟล์ `.git` หลักที่ Root ของโปรเจกต์เท่านั้น.
- ตรวจสอบไฟล์ `.gitignore` พบว่ามีการตั้งค่า Block โฟลเดอร์ที่เป็นปัญหาเรียบร้อยแล้ว.

## 5. Prevention (แนวทางป้องกัน)

- **Restart Recommendation:** เมื่อเกิดเหตุการณ์ Git ซ้อน ควร Restart IDE ทันทีเพื่อให้ Agent เคลียร์สถานะในหน่วยความจำ.
- **Git Hygiene:** หลีกเลี่ยงการเปิด IDE สองตัว (เช่น Antigravity และ Claude Code) ที่มีการสร้าง Worktree ซ้อนกันในโฟลเดอร์เดียวกันโดยไม่ตั้งค่า Ignore.
