---
title: FRAMEWORK--HIERARCHY-COMPACTION-STANDARDS
summary: มาตรฐานการบีบอัดไฟล์กายภาพ (Chain-Driven Atom Compaction) และการกำหนดขอบเขตบริบทของ AI Agent (Local Graph Hop Scaling H0-H5)
doc_id: GVDOC-1003
created: "2026-06-02T19:40:00+07:00,Boss(CEO)"
updated: "2026-06-07T12:20:00+07:00,Boss(CEO),98db9a5"
version: "1.3.0b"
state: active
type: framework
vault_id: default
source_type: axiomatic
tags:
  - architecture
  - compaction
  - scaling
  - graph
  - framework
aliases:
  - "FRAMEWORK--"
  - "FRAMEWORK::"
  - "FRAMEWORK::HIERARCHY-COMPACTION-STANDARDS"
role: Governance / architectural framework
block_manifest:
    genesisblock: [[GENESIS::GoVibe-CoDev-Standard-FRAMEWORK]]
    masterblock: [[MASTER::Multi-Platform-ARCHITECTURE-FRAMEWORK]]
wikilink:  
crosslink:

---

# FRAMEWORK::HIERARCHY-COMPACTION-STANDARDS

**Hierarchy: Chain-Driven Atom Compaction Model & Local Graph Scaling**
เอกสารมาตรฐานการจัดลำดับขอบเขตไฟล์ระดับกายภาพ (On-Disk) และระดับตรรกะ (In-Memory Graph)
ออกแบบมาเพื่อจำกัดขอบเขต Context และแก้ไขปัญหา Disk I/O Bottleneck ในโปรเจกต์ Enterprise Scale

---

## 1. บทนำ (Introduction)
ในการทำระบบ **Doc-Driven Development (DDD)** และ **Diagram-to-Code** ระดับ Enterprise ที่มีขนาดความต้องการสูง ปัญหาคลาสสิกที่พบคือ **Inode Exhaustion, File I/O Bottleneck, และ Git Graph Fragmentation** ที่เกิดจากการมีไฟล์ขนาดเล็ก (1-2 KB) กระจัดกระจายเป็นหมื่นๆ ไฟล์บนฮาร์ดดิสก์

**Chain-Driven Atom Compaction Model** แก้ปัญหานี้โดยใช้หลักการ **"Compound Document"** หรือการยุบรวม Node ที่อยู่ในสายสัมพันธ์การทำงานเดียวกัน (Execution Chain) ให้บันทึกอยู่บน **1 ไฟล์กายภาพเดี่ยว (1 Physical File)** แต่เมื่อเข้าสู่ขั้นตอนการประมวลผลระบบกราฟ (GKS Parser Engine) จะแยกสับออกมาเป็น Node ย่อยๆ ในเมมโมรีตามระดับความลึกที่เลือกใช้งาน

---

## **2. มาตรฐานระดับความลึกการบีบอัดไฟล์ (Compaction Heights: H5 - H1)**
การเลือกใช้งานความสูง (Height) จะเป็นตัวกำหนดว่าใน 1 ไฟล์จะมีการซ้อนทับกันกี่ระดับชั้น โดยแบ่งออกตามความซับซ้อนของแต่ละ System ดังนี้:

### **📊 สรุปความสัมพันธ์ (Hierarchy Resolution Map)**
* **H5 (3 Layers)**  ➔ `[L2-System] ➔ [L1-Module] ➔ [L0-Function]`
* **H4 (4 Layers)**  ➔ `[L3-System] ➔ [L2-Module] ➔ [L1-Feat] ➔ [L0-Function]`
* **H3 (5 Layers)**  ➔ `[L4-System] ➔ [L3-Module] ➔ [L2-Feat] ➔ [L1-Component] ➔ [L0-Method]`
* **H2 (6 Layers)**  ➔ `[L5-System] ➔ [L4-Module] ➔ [L3-Sub-Module] ➔ [L2-Feat] ➔ [L1-Component] ➔ [L0-Method]`
* **H1 (8 Layers)**  ➔ `[L7-System] ➔ [L6-Sub-System] ➔ [L5-Module] ➔ [L4-Sub-Module] ➔ [L3-Feat] ➔ [L2-Component] ➔ [L1-Class] ➔ [L0-Method]`

---

## **3. 🪐 Context Scaling Tiers (Graph Database & Small World Phenomenon)**

ตามทฤษฎีนี้ระบุว่า *'Node ทุกตัวในเครือข่ายสามารถเชื่อมต่อถึงกันได้ภายใน 6 ก้าว'* ดังนั้นเราจึงสร้าง **"Scaling Tier"** เพื่อจำกัดวง (Local Graph Mode) ของ Agent ไว้สูงสุดที่ **5 Hops (รวมตัวมันเอง = 6 Nodes)** ซึ่งพิสูจน์ได้ทางคณิตศาสตร์แล้วว่าเพียงพอต่อการเข้าถึง Context ทั้งโปรเจกต์โดยไม่ต้องโหลดไฟล์ทั้งหมด:

*   **H0 - Subtasks / Pull Requests (0 Hop: Quick Task)** 
    *   **ลักษณะงาน:** งานย่อยเล็กๆ, Hotfix, แก้ไขคำผิด, เขียน Unit Test หรือขั้นตอนสุดท้ายของการยิง Code ลง Branch และส่งสร้าง Pull Request (PR)
    *   **บริบทที่ใช้:** `0 Hop` (มองเห็นแค่ Node ตนเอง) โฟกัสเฉพาะไฟล์เดี่ยวที่ระบุโดยตรงเท่านั้น ทำงานแบบมุ่งเป้าแบบไม่มี Context รอบตัว
    *   **Workflow:** จบได้ทันทีโดยไม่ต้องร่างแผนการ (No Plan Required)
*   **H1 - Tasks (1 Hop: Component Assembly)** 
    *   **ลักษณะงาน:** การเขียนโค้ดและสร้าง Component จริงในระดับปฏิบัติการ (Code Implementation)
    *   **บริบทที่ใช้:** `1 Hop` ดึงข้อมูล Node ตัวเองบวกกับไฟล์นำเข้า/ส่งออก (Imports/Exports) ที่อยู่ติดกัน 1 ระดับชั้นรอบตัว
    *   **Workflow:** Teammate (T2/T1) ใช้ระบบ Self-Claiming ดึงงานและประกาศ File Lock เพื่อลงมือทำ
*   **H2 - Stories / Specs (2 Hops: Feature Assembly)** 
    *   **ลักษณะงาน:** สเปกหรือฟีเจอร์ย่อยที่เน้นมุมมองผู้ใช้ (User Stories / Technical Specs)
    *   **บริบทที่ใช้:** `2 Hops` สแกนครอบคลุมโฟลเดอร์ฟีเจอร์รวมถึงประเภทข้อมูล (Types) และ API บริเวณใกล้เคียงทั้งหมด
    *   **Workflow:** Agent T3 (Lead) เป็นคนวางแผนและจัดการ Plan Approval ร่วมกับ USER
*   **H3 - Epics (3 Hops: Module Integration)** 
    *   **ลักษณะงาน:** ฟีเจอร์ย่อยระดับโมดูลหลัก (เช่น ระบบชำระเงิน, ระบบตะกร้าสินค้า)
    *   **บริบทที่ใช้:** `3 Hops` วิเคราะห์ผลกระทบระดับโมดูลข้างเคียงเพื่อให้สถาปนิกคำนวณการหั่นแบ่งแยกชิ้นส่วนงาน
    *   **Workflow:** มอบหมายให้ Agent T3 (Lead) ในการวิเคราะห์และแจกจ่ายงานย่อย
*   **H4 - Phase / Theme (4 Hops: System Architecture)** 
    *   **ลักษณะงาน:** ทิศทางสถาปัตยกรรมหลัก หรือการสลับโครงสร้างรากฐานขนาดใหญ่ (เช่น การเปลี่ยนระบบ ORM / Database)
    *   **บริบทที่ใช้:** `4 Hops` สแกนตรวจสอบความเกี่ยวโยงของสถาปัตยกรรมระบบ ตั้งแต่ระดับล่างสุดไปจนถึงการเช็คขอบเขต System
*   **H5 - Masterplan / Roadmap (5 Hops: Enterprise Vision)** 
    *   **ลักษณะงาน:** ทิศทางและแผนงานระยะยาวระดับองค์กร (Vision & Roadmap) ที่ส่งผลต่อทุกระบบในบริษัท
    *   **บริบทที่ใช้:** `5 Hops` ครอบคลุมฐานความรู้ทั้งหมด (GKS) เพื่อหาจุดกระทบข้ามระบบ (Cross-System Refactoring)
    *   **Workflow:** ดูแลจัดการโดยมนุษย์ (USER) เป็นผู้ควบคุมหลักในการบริหารความเสี่ยง

> [!TIP]
> กฎ 6 Nodes (H0 ถึง H5) คือมาตรฐานที่อ้างอิงจาก **Small World Phenomenon**: หากงานใดในระบบของคุณต้องวิเคราะห์ลึกเกิน 5 Hops เพื่อที่จะเข้าใจความสัมพันธ์ แสดงว่าสถาปัตยกรรมของคุณไม่ได้เป็นแบบ Small World Network แต่เป็น Spaghetti Code ที่มีการผูกขาด (Coupling) ผิดปกติ และจำเป็นต้อง Refactoring ทันที

### **3.1 โครงสร้างลำดับชั้นการทำงานแบบสากล (Work Hierarchy & Agile Alignment)**
เพื่อให้ระบบ GoVibe เป็นสากลและไม่สับสนกับทฤษฎีการจัดการยุคใหม่ เราจึงแยกโครงสร้างการทำงานและกรอบเวลาออกจากกันอย่างเด็ดขาดตามมาตรฐานอุตสาหกรรม (WBS vs Timebox):

* **แกนที่ 1: แกนเนื้องาน (Work Breakdown Structure - WBS)**
  นี่คือโครงสร้างความลึกของสเกลงานจากใหญ่ไปเล็ก:
  1. **Masterplan / Vision** (ภาพรวมและเป้าหมายสูงสุด)
  2. **Roadmap / Initiative** (แผนนำทางการพัฒนาและทิศทางรวม)
  3. **Phase / Theme** (ช่วงเวลาการปล่อย หรือกลุ่มฟังก์ชันเป้าหมายหลัก)
  4. **Epic** (ฟีเจอร์ใหญ่ที่ต้องใช้เวลาทำนาน เช่น "ระบบลงทะเบียนสมาชิก")
  5. **Story / Feature** (ความต้องการใช้งานในมุมมองผู้ใช้และรายละเอียด Spec)
  6. **Task** (เนื้องานจริงเชิงเทคนิคที่ต้องทำ เช่น "สร้าง API endpoint สำหรับ register")
  7. **Subtask** (หน่วยงานย่อยที่สุดเพื่อเก็บความสะอาด เช่น "เขียน unit test ครอบคลุม error cases")

* **แกนที่ 2: แกนการจัดการเวลา (Time Management)**
  ถังเวลาที่ใช้ในการจำกัดและส่งมอบงานในแกนเนื้องาน:
  1. **Release Plan:** แผนส่งมอบเวอร์ชันใหญ่ของซอฟต์แวร์ (เช่น `v1.0.0`)
  2. **Sprint / Cycle:** รอบช่วงเวลาการทำงานจำกัด (1-2 สัปดาห์) ซึ่งจะใช้วิธี **ดึง (Pull)** งานประเภท **Task** หรือ **Story** จากแกนที่ 1 เข้ามาวางเป็น Backlog ประจำรอบเวลาทำงานนั้นๆ

---

## **4. กฎสถาปัตยกรรมและการแปลงข้อมูล (Parser Engine Protocol)**
เพื่อให้ระบบกราฟ (Genesis Block Graph Backend) และดัชนี L0 (`atomic_index.jsonl`) ทำงานได้อย่างราบรื่น ตัวแปลงสัญญาณ (GKS Parser) จะทำงานดังนี้:

1.  **State Partitioning:** ระบบจะสแกนหาตัวแบ่งพาร์ติชันคือกิ่งหัวข้อ Markdown `^#\s.+\s\[L\d-.+\]\s([A-Z0-9_--]+)` เพื่อขึ้นรูปอะตอมย่อยแบบ Virtual อัตโนมัติ
2.  **Deterministic Backlink Injection:** ตัวแปรสิทธิ์การทำงาน (YAML/JSON Block) ในแต่ละระดับชั้น จะได้รับการฉีดพ่นค่าความสัมพันธ์ `crosslinks` วิ่งขนานย้อนคืนสายโซ่ขึ้นไปทีละลำดับชั้นโดยผู้ดูแลระบบคอมไพล์ เพื่อป้องกันการเกิดปัญหาหักวงจรแบบลูป (Acyclic Invariant Enforcement)
3.  **Block Overwrite Mechanism:** เมื่อ AI สั่งอัปเดตระบบในระดับ `L0` หรือ `L1` ระบบจำเพาะเจาะจงล็อกเป้าหมายเฉพาะช่วงของส่วนหัวข้อที่แก้ไขและทำการเขียนเนื้อหาเปลี่ยนถ่ายสอดไส้ข้อมูลกลับเข้าไปในตำแหน่งไฟล์กายภาพเดิมอย่างแม่นยำ โดยรักษาข้อมูลของระดับอื่นไว้ครบถ้วน 100%

---

## **5. CHANGELOG**

| Version | Date | Status | Summary |
|---|---|---|---|
| 1.3.0b | 2026-06-07 | active | ทำการวิเคราะห์และแยกแกนเนื้องาน (WBS) ออกจากแกนเวลา (Sprint/Cycle) และปรับการแมป H0-H5 ให้ตรงตามมาตรฐาน Agile |
| 1.2.0b | 2026-06-07 | active | เพิ่มการเชื่อมโยงระบบแกนเวลา (Sprint/Cycle) เข้ากับ Hop H0-H5 และปรับโครงสร้างหัวข้อย่อย |
| 1.0.0b | 2026-06-02 | active | ร่างโครงสร้างมาตรฐานการบีบอัดข้อมูลระบบและ Hierarchy Compaction รุ่นแรก |
