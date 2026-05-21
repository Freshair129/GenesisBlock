# INCIDENT REPORT: Antigravity Agent Failure (Git Config Conflict)

**Date:** 2026-05-13
**Status:** Resolved (Resolved via Git Config Correction)
**Severity:** High (Agent Functionality Blocked)

## 1. Symptoms (อาการ)

- Antigravity Agent ไม่ทำงาน (Fail to start).
- แถบสถานะแจ้งเตือน `LanguageServerClient must be initialized first!`.
- ใน Log (`ls-main.log`) พบข้อความ Error: `core.repositoryformatversion does not support extension: worktreeconfig`.
- ตัว Agent ไม่สามารถเข้าถึงบริบทของไฟล์ (Context) และเครื่องมือ (Tools) ได้.

## 2. Root Cause (ที่มาของปัญหา)

ปัญหานี้เกิดจาก **"Git Configuration Inconsistency"**:

- **Extension Conflict:** มีการเปิดใช้งาน `extensions.worktreeConfig=true` ใน `.git/config` แต่อยู่ภายใต้ `core.repositoryformatversion = 0`.
- **Git Specification:** ตามมาตรฐานของ Git หากมีการใช้ `extensions` รุ่นของ Repository Format ต้องถูกตั้งเป็น `1` เท่านั้น.
- **Language Server Crash:** เมื่อ Language Server ของ Antigravity ตรวจพบความขัดแย้งนี้ขณะพยายามอ่านโครงสร้างโปรเจกต์ มันจะหยุดทำงาน (Crash) ทันที ส่งผลให้ Service อื่นๆ (เช่น `agentSessions`) ไม่สามารถเริ่มต้นได้.
- **Stale Processes:** การ Restart IDE เพียงอย่างเดียวไม่เพียงพอ เนื่องจากมี Process เบื้องหลังค้างอยู่ในหน่วยความจำที่ยังถือคอนฟิกเดิมไว้.

## 3. Resolution (การแก้ไข)

1. **Git Format Update:** ปรับปรุงรุ่นของ Repository Format เป็น `1` ด้วยคำสั่ง `git config core.repositoryformatversion 1`.
2. **Extension Deactivation:** ทำการยกเลิก (Unset) `extensions.worktreeConfig` เพื่อลดความซับซ้อนที่ Language Server ไม่รองรับ.
3. **Process Force-Kill:** สั่งปิดทุก Process ของ Antigravity ในระบบ (Windows Task) เพื่อล้างสถานะที่ค้างอยู่ (Stale state).
4. **Redundancy Cleanup:** ลบไฟล์ `package-lock.json` ส่วนเกินใน Package ย่อยที่อาจทำให้การวิเคราะห์ Dependency ผิดพลาด.
5. **Clean Restart:** ให้ผู้ใช้ทำการ Restart โปรแกรมเพื่อให้ระบบโหลดคอนฟิกใหม่ที่ถูกต้อง.

## 4. Verification (การยืนยัน)

- ตรวจสอบ `ls-main.log` ล่าสุดพบว่า Language Server สามารถ Initialize ได้สำเร็จ.
- ผู้ใช้ยืนยันว่า Agent กลับมาใช้งานได้ตามปกติ.

## 5. Prevention (แนวทางป้องกัน)

- **Git Consistency Check:** หากมีการใช้ Worktree หรือฟีเจอร์ขั้นสูงของ Git ต้องตรวจสอบว่า `repositoryformatversion` สอดคล้องกัน.
- **GEMINI.md Update:** เพิ่มคำเตือนใน `GEMINI.md` เรื่องความขัดแย้งของ Git Config และความสำคัญของการตรวจสอบ Process เบื้องหลัง.
- **Monorepo Hygiene:** ยึดถือไฟล์ Lock ที่ Root เพียงชุดเดียวเพื่อป้องกันความสับสนของ Agent.
