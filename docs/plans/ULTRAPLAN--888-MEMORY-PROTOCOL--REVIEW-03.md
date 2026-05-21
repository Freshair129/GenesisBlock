# Review #03 — ULTRAPLAN--888-MEMORY-PROTOCOL

> **Reviewer:** Gemini (model: **Gemini 1.5 Flash**)
> **Role:** Independent Reviewer (Third Opinion)
> **Reviewed file:** `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md`
> **Date:** 2026-05-17T10:00:00+07:00
> **Verdict:** **Approved with minor suggestions** — the plan is robust and well-aligned with the project's architectural principles.

---

## 0. Overview of the Cognitive System Project

โครงการ Cognitive System [1] เป็นสถาปัตยกรรมแบบ Multi-Agent ที่ขับเคลื่อนด้วยหลักการ Doc-Before-Code โดยมีเป้าหมายในการสร้าง "สายพานการผลิตข้อมูล" (Information Assembly Line) เพื่อแปลงแนวคิดของมนุษย์ไปสู่โค้ดที่รันได้จริง [2]. ระบบนี้ออกแบบมาเพื่อใช้ประโยชน์จากความสามารถที่แตกต่างกันของ Large, Medium และ Small Local LLM ในการทำงานร่วมกันอย่างมีประสิทธิภาพ โดยเน้นการแยกบริบท (Context Isolation) เพื่อให้ได้ความแม่นยำสูงและลดต้นทุน [2].

โครงสร้างหลักของระบบประกอบด้วย 5 เสาหลัก [2]:

1.  **Agent Layer:** แต่ละ Agent (เช่น Claude Code, Gemini CLI, Qwen CLI) มี identity และ scratchpad ของตัวเอง ทำงานแยกกันใน directory เฉพาะ [1].
2.  **Manager — MSP (Memory & Soul Passport):** ทำหน้าที่เป็น Gatekeeper สำหรับความรู้ระยะยาวทั้งหมดที่เข้าสู่ GKS โดยมีการตรวจสอบความถูกต้อง (Validation) และจัดการกระบวนการ Candidate flow ซึ่ง Agent จะต้องเสนอผ่าน `msp_candidate` MCP tool และรอการอนุมัติจากมนุษย์ผ่าน Pull Request ก่อนที่จะ Promote เข้าสู่ GKS [2]. MSP ยังใช้รูปแบบ Smart Proxy (Hexagonal) เพื่อแยกส่วน Interface, Orchestrator, Clients และ Domain อย่างชัดเจน [2].
3.  **Storage — GKS (Genesis/Global Knowledge System):** เป็นคลังความรู้ระยะยาว (Single Source of Truth - SSOT) ที่เก็บ Atomic Note ในรูปแบบ Markdown และ JSONL index [2]. GKS มีการจัดหมวดหมู่ Atom ตาม `atom_registry.yaml` ซึ่งเป็นแหล่งข้อมูลหลักสำหรับประเภทของ Atom [3].
4.  **Runtime:** ส่วนที่รันโค้ดที่สร้างขึ้น.
5.  **Observability:** ส่วนสำหรับการตรวจสอบและติดตามสถานะของระบบ.

โครงการนี้ยังเน้นย้ำถึงกฎการทำงานร่วมกันของ Agent, กฎสภาพแวดล้อม, การจัดการ Git และกระบวนการ Doc-Before-Code ที่เข้มงวด โดยกำหนดให้มีการสร้าง Artifacts ตามลำดับ (CONCEPT → ADR/FEAT → BLUEPRINT → Code → AUDIT) ก่อนการเขียนโค้ดจริง [1].

## 1. Analysis of ULTRAPLAN--888-MEMORY-PROTOCOL

เอกสาร `ULTRAPLAN--888-MEMORY-PROTOCOL.md` และรีวิวแรกโดย Claude Code [4] แสดงให้เห็นถึงการวิเคราะห์ปัญหาที่ชัดเจนและข้อเสนอแนะที่สอดคล้องกับหลักการของ Cognitive System. แผนนี้มุ่งเน้นการแก้ไขช่องว่างทางสถาปัตยกรรมที่สำคัญคือ "cross-session distillation" และ "confidence-as-data" ซึ่งเป็นส่วนที่ขาดหายไปจากโมดูล `compressor` และ `consolidator` ที่มีอยู่ [4].

### 1.1 Strengths

*   **การระบุปัญหาที่ชัดเจน:** แผนระบุช่องว่างของระบบที่เกี่ยวข้องกับการกลั่นกรองข้อมูลข้ามเซสชันและความมั่นใจของข้อมูลได้อย่างแม่นยำ.
*   **สอดคล้องกับหลักการ Doc-Before-Code:** การเสนอให้สร้าง Atom ใหม่ (CONCEPT, ADR, BLUEPRINT) ก่อนการเขียนโค้ดสอดคล้องกับปรัชญาหลักของโครงการ [1, 4].
*   **การใช้ประโยชน์จาก Agent อื่นๆ:** แผนมีการอ้างอิงถึงการทำงานร่วมกับ Agent อื่นๆ เช่น Gemini ในส่วนของ `consolidator` และการกำหนดขอบเขตความรับผิดชอบที่ชัดเจน [4].
*   **การประเมินความเสี่ยงและเงื่อนไขการอนุมัติ:** มีการระบุความเสี่ยงและเงื่อนไขที่ต้องแก้ไขก่อนเริ่ม Phase 1 อย่างชัดเจน ซึ่งเป็นแนวทางปฏิบัติที่ดีในการบริหารจัดการโครงการ [4].
*   **การพิจารณาด้านต้นทุน:** การกล่าวถึงการประหยัด token และลด hallucination ใน `FRAMEWORK_MASTER_SPEC.md` [2] และการพิจารณา LLM cost per Core distillation ในรีวิว [4] แสดงให้เห็นถึงความตระหนักในด้านประสิทธิภาพและต้นทุน.

### 1.2 Areas for Further Consideration / Minor Suggestions

จากรีวิวของ Claude Code [4] และการวิเคราะห์เพิ่มเติม ผมมีข้อเสนอแนะดังนี้:

*   **การติดตามและประเมินผล (Telemetry):** ข้อเสนอแนะ C ในรีวิวแรก [4] ที่ให้เพิ่ม Telemetry สำหรับ `distiller.session_to_core.count`, `distiller.core_to_sphere.count`, `distiller.belief_revision.downgrade.count` และ `distiller.llm.cost_usd` เป็นสิ่งสำคัญอย่างยิ่ง. การมีข้อมูลเหล่านี้จะช่วยให้การประเมินผล Phase C-D เป็นไปอย่างมีข้อมูลและแม่นยำ ไม่ใช่การคาดเดา.
*   **ความชัดเจนของขอบเขต (Boundary Definition):** ข้อเสนอแนะให้เพิ่ม `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` [4] เป็นสิ่งจำเป็นเพื่อป้องกันความสับสนและข้อขัดแย้งในอนาคตเกี่ยวกับขอบเขตความรับผิดชอบของโมดูล `distiller` และ `consolidator`.
*   **การจัดการ `package-lock.json`:** `AGENT.md` [1] ระบุว่าควรมี `package-lock.json` เพียงไฟล์เดียวที่ root ของ repo เพื่อหลีกเลี่ยงความสับสนของ Antigravity. ควรตรวจสอบให้แน่ใจว่าแนวทางปฏิบัตินี้ได้รับการปฏิบัติตามอย่างเคร่งครัดในทุกการพัฒนา.
*   **การอัปเดตเอกสารอย่างต่อเนื่อง:** `FRAMEWORK_MASTER_SPEC.md` [2] มีการกล่าวถึงการอัปเดต `atom_registry.yaml` แทน `docs/gks/KNOWLEDGE-TYPES.md` และการเปลี่ยนชื่อ Prefix บางตัว. ควรมีการตรวจสอบและอัปเดตเอกสารที่เกี่ยวข้องทั้งหมดให้สอดคล้องกันเพื่อรักษา SSOT.

## 2. Recommendations

โดยรวมแล้ว `ULTRAPLAN--888-MEMORY-PROTOCOL` เป็นแผนงานที่แข็งแกร่งและสอดคล้องกับวิสัยทัศน์ของโครงการ Cognitive System. ผมขอแนะนำให้ดำเนินการตามแผนนี้ โดยให้ความสำคัญกับการแก้ไข Issue #1 และ #2 ตามที่ Claude Code แนะนำ [4] และพิจารณาข้อเสนอแนะเพิ่มเติมดังนี้:

1.  **ดำเนินการแก้ไข Issue #1 และ #2:** ตรวจสอบและแก้ไข `gemini/m7b-consolidator TS errors` และเพิ่ม `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` เข้าไปใน Phase 2 ของแผน [4].
2.  **เพิ่ม Telemetry:** บูรณาการการเก็บ Telemetry ที่จำเป็นสำหรับการประเมินผล `distiller` เข้าไปใน `BLUEPRINT--TIERED-DISTILLATION` เพื่อให้สามารถวัดผลและตัดสินใจใน Phase C-D ได้อย่างมีประสิทธิภาพ.
3.  **ทบทวนและอัปเดตเอกสาร:** ตรวจสอบให้แน่ใจว่าเอกสารทั้งหมดในโครงการ (โดยเฉพาะที่เกี่ยวข้องกับ Atom Taxonomy และ Monorepo Layout) ได้รับการอัปเดตให้สอดคล้องกับการเปลี่ยนแปลงล่าสุดใน `atom_registry.yaml` และ `FRAMEWORK_MASTER_SPEC.md`.
4.  **รักษาความสอดคล้องของ `package-lock.json`:** เน้นย้ำให้ทีมพัฒนาปฏิบัติตามกฎการมี `package-lock.json` เพียงไฟล์เดียวที่ root ของ repository.

---

*— ความเห็นลำดับที่ 3 โดย Gemini, model `Gemini 1.5 Flash`.* 

## References

[1] Freshair129/cognitive_system. `AGENT.md`. [https://github.com/Freshair129/cognitive_system/blob/main/AGENT.md](https://github.com/Freshair129/cognitive_system/blob/main/AGENT.md)
[2] Freshair129/cognitive_system. `FRAMEWORK_MASTER_SPEC.md`. [https://github.com/Freshair129/cognitive_system/blob/main/FRAMEWORK_MASTER_SPEC.md](https://github.com/Freshair129/cognitive_system/blob/main/FRAMEWORK_MASTER_SPEC.md)
[3] Freshair129/cognitive_system. `atom_registry.yaml`. [https://github.com/Freshair129/cognitive_system/blob/main/atom_registry.yaml](https://github.com/Freshair129/cognitive_system/blob/main/atom_registry.yaml)
[4] Freshair129/cognitive_system. `ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01.md`. [https://github.com/Freshair129/cognitive_system/blob/main/docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01.md](https://github.com/Freshair129/cognitive_system/blob/main/docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01.md)
