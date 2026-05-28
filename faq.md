Viewed FRAMEWORK--SYMBOL-GRAPH.md:1-139

ระบบมีการออกแบบและนำเทคโนโลยีเหล่านี้มาใช้งานจริงเป็นสองส่วนขนานกัน คือ **มิติด้านเนื้อหา (Conceptual Axis - Atoms Graph)** และ **มิติด้านโครงสร้างโค้ด (Structural Axis - Symbol Graph)** โดยอ้างอิงตามข้อกำหนดของสถาปัตยกรรมใน [FRAMEWORK--SYMBOL-GRAPH](file:///c:/Users/freshair/cognitive_system/gks/framework/FRAMEWORK--SYMBOL-GRAPH.md) สรุปคำตอบสำหรับคำถามทั้ง 6 ข้อดังนี้ครับ:

---

### 1. มีการทำ Symbol Linking ไหม?
* **มีครับ** 
* อะตอมใน GKS (Conceptual) และสัญลักษณ์จริงในโค้ด (Structural) จะถูกเชื่อมโยงผ่านฟิลด์ **`linked_symbols`** ใน YAML frontmatter ของเอกสาร อะตอมแต่ละตัวจะชี้ไปยังไฟล์หรือฟังก์ชันโค้ดที่รันจริง (เช่น `packages/msp/src/validator/proto/genesis-block-membership.ts` ใน `PROTO--GENESIS-BLOCK-MEMBERSHIP`) เพื่อทำหน้าที่เป็นพิกัดอ้างอิงระหว่างสเปกและโค้ด

### 2. Dependency Map ของ Doc มีไหม?
* **มีครับ** 
* คลังความรู้ GKS จะใช้คุณลักษณะ **`crosslinks`** ใน frontmatter (เช่น `references`, `implements`, `parent_blueprint`, `supersedes`, `related_to`, `depends_on`) ในการจัดทำ Dependency Map
* ความสัมพันธ์เหล่านี้จะถูกประมวลผลด้วยคำสั่ง `npm run msp:index` ออกมาเป็นไฟล์ `gks/00_index/atomic_index.jsonl` และแสดงผลภาพรวมเป็น **Obsidian Graph View** สำหรับมนุษย์ และช่วยให้ตัวตรวจสอบความถูกต้องของลิงก์ (`npm run msp:check-links`) ค้นหาจุดที่พังได้ทันที

### 3. การทำ Impact Analysis เพื่อหา Blast Radius ของ Code และ Doc
* **มีครับ** การหาผลกระทบและ Blast Radius ดำเนินการผ่านความร่วมมือของสองระบบ:
  * **Code-to-Code**: ระบบสร้าง Call Graph ขึ้นมาเพื่อหาความสัมพันธ์ของโค้ด เช่น `calls`, `extends`, `implements`, `imports` ซึ่งเมื่อเราแก้โค้ดที่ฟังก์ชันใดระบบจะคำนวณย้อนกลับ (Reverse-call closure lookup) เพื่อหา Blast Radius ของฟังก์ชันทั้งหมดที่เรียกใช้งาน
  * **Code-to-Doc (และกลับกัน)**: เมื่อเกิดผลกระทบที่ระดับโค้ด ระบบจะค้นหาผ่าน `linked_symbols` ว่าฟังก์ชัน/ไฟล์ที่มีปัญหาเชื่อมไปหาเอกสาร Feature Spec (`FEAT--`) หรือ Protocol ตัวใด และระบุขอบเขตการเปลี่ยนสเปกที่ตามมาได้ทันที
  * **รูปแบบการเก็บ Link**:
    * **Doc**: จัดเก็บเป็น **YAML frontmatter** (wikilinks `[[ID]]` และ array ของ `linked_symbols` เช่น `file: path.ts`)
    * **Code**: จัดเก็บใน **GenesisGraphBackend** (Embedded Graph DB ที่ใช้ Rust-based napi-rs ในรูปแบบ SQLite + JSONL log) โดยผูกสัมพันธ์ด้วย Typed Edges (calls, extends, etc.)

### 4. มีการใช้ Leiden Algorithm จัดกลุ่ม Community จริงไหม?
* **จริงครับ** 
* ในการประมวลผลขั้นที่ 6 (Abstract Stage) ของ Symbol Graph Pipeline จะมีการใช้ **Leiden Algorithm** เพื่อจัดสัญลักษณ์ของโค้ดที่มีความเกี่ยวข้องกันในการประมวลผล (ตาม Call Graph) เข้าด้วยกันเป็น "Communities" ทำให้ AI และผู้พัฒนาสามารถเห็นภาพรวมของกลุ่มโค้ดได้ตามพฤติกรรมและความรับผิดชอบจริง (เช่น Core Utils, Middleware, API handlers) แทนที่จะมองเห็นแค่พิกัดของไฟล์ในโฟลเดอร์

### 5. Object-Relational Mapping (ORM)?
* **มีครับ** 
* ในการประมวลผลขั้นที่ 4 (Framework Stage) ของ Symbol Graph Pipeline จะทำการวิเคราะห์ **ORM Mapping** (เฉพาะเจาะจงที่ Prisma และ Supabase) เพื่อตรวจจับและเชื่อมโยงความสัมพันธ์ของ Database schemas และ Entity models ไปหาตัวแปรหรือฟังก์ชันที่สืบทอด/เรียกใช้งานในโค้ด TypeScript

### 6. MRO (Method Resolution Order)?
* **มีครับ** 
* ในการประมวลผลขั้นที่ 5 (Relationship Stage) ของ Symbol Graph Pipeline จะทำการวิเคราะห์ **MRO (Method Resolution Order / Heritage Chain)** เมื่อคลาสย่อยทำการ extend หรือ implement อินเตอร์เฟซต่างๆ เพื่อช่วยให้ตัว Call Graph สามารถแกะลำดับขั้นของการสืบทอดและระบุเป้าหมายปลายทางของ Method ที่ถูกทับซ้อน (Override) หรือเรียกใช้งานข้ามคลาสได้อย่างแม่นยำครับ