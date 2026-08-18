# Genesis Block — Issues, Concerns and Intended Solutions

## 1. ข้อมูลของระบบกระจายอยู่หลาย Storage Engine

* **Issue / Concern**

  * ระบบ AI หรือ Agent หนึ่งระบบมักต้องใช้ SQLite หรือ PostgreSQL สำหรับข้อมูลโครงสร้าง, Vector Database สำหรับ embedding, Graph Database สำหรับความสัมพันธ์ และ File System สำหรับไฟล์จริง
  * แต่ละระบบมี identity, transaction และ recovery mechanism ของตัวเอง
  * Application ต้องเป็นผู้รักษาความสัมพันธ์ระหว่างข้อมูลในแต่ละ storage เอง
  * เมื่อ transaction บางส่วนสำเร็จและบางส่วนล้มเหลว อาจเกิด orphan data หรือ state ไม่ตรงกัน

* **เหตุผลที่เสนอ**

  * GenesisBlockDB ควรเป็น authority กลางของ state, identity, version และ transaction
  * Relational, graph, vector, file metadata และ provenance ควรถูกผูกอยู่ใน logical transaction เดียวกัน
  * Application ไม่ควรต้องสร้าง orchestration layer เพื่อเชื่อม database หลายชนิดเองทุกโครงการ

* **ผลลัพธ์ที่ต้องการ**

  * มี Unified Transaction ที่รองรับ relational mutation, graph mutation, vector mutation, object reference, file version และ temporal metadata
  * ทุกข้อมูลอ้างอิง stable entity identity ชุดเดียวกัน
  * เมื่อเกิด failure ระบบสามารถ replay, rollback หรือซ่อม projection ให้กลับสู่ state ที่ถูกต้องได้

---

## 2. WAL และ Recovery ยังต้องแข็งแรงก่อนเพิ่มระบบ Filesystem

* **Issue / Concern**

  * เมื่อ GenesisBlockDB ต้องรับผิดชอบ file state, object state และ application state ความเสียหายจาก transaction ที่ไม่สมบูรณ์จะรุนแรงกว่า database metadata ทั่วไป
  * หาก process ถูก kill ระหว่างเขียนไฟล์, อัปเดต namespace หรือสร้าง vector index อาจเหลือข้อมูลเพียงบางส่วน
  * ถ้า recovery ไม่ deterministic ระบบอาจไม่รู้ว่า object ใด committed แล้วหรือเป็นเพียง staging file

* **เหตุผลที่เสนอ**

  * Signed WAL ต้องเป็น durability authority ที่ชัดเจน
  * Projection เช่น SQLite, graph index, vector index และ namespace index ควรถูก rebuild หรือซ่อมจาก WAL ได้
  * ต้องแยกสถานะ staged, committed, promoted และ orphaned อย่างชัดเจน

* **ผลลัพธ์ที่ต้องการ**

  * WAL durability และ idempotent replay
  * Stable commit frontier
  * Checkpoint, snapshot และ compaction
  * Staging cleanup และ orphan detection
  * Fault-injection tests สำหรับ crash ทุกช่วงของ transaction
  * ปิดโปรแกรมหรือ kill process ระหว่าง commit แล้วข้อมูลไม่สูญหายหรือค้างใน state ที่ตีความไม่ได้

---

## 3. ไม่ควรเก็บไฟล์ขนาดใหญ่ทั้งหมดเป็น BLOB ใน SQLite

* **Issue / Concern**

  * การรวมไฟล์ขนาดใหญ่ทั้งหมดไว้ใน database file เดียวเพิ่ม corruption blast radius
  * Backup, compaction และการเขียนไฟล์บางส่วนอาจกระทบฐานข้อมูลก้อนใหญ่
  * Model, media, archive และ binary artifacts ต้องการ streaming และ range read
  * SQLite ไม่ควรต้องรับภาระเป็นทั้ง relational projection และ large-object filesystem

* **เหตุผลที่เสนอ**

  * Genesis Block ควรมี Managed Object Store แยกจาก SQLite projection
  * Object Store ควรเป็นผู้เก็บ bytes ส่วน GenesisBlockDB เก็บ identity, metadata, version, relationships และ lifecycle
  * Physical storage สามารถแบ่งเป็น object files, pack files หรือ segmented storage ได้

* **ผลลัพธ์ที่ต้องการ**

  * Content-addressed objects
  * Streaming read/write
  * Range read
  * Large-object support
  * Integrity checksum
  * Compression และ optional encryption
  * Garbage collection
  * Storage tier locator
  * Replication และ backup state
  * One logical Genesis unit แต่ไม่บังคับให้เป็น physical `.db` ไฟล์เดียว

---

## 4. Path ไม่เหมาะใช้เป็น Identity ของไฟล์

* **Issue / Concern**

  * Path เปลี่ยนได้เมื่อ rename, move, mount คนละไดรฟ์ หรือ restore หลังลง Windows ใหม่
  * ไฟล์เดียวกันอาจปรากฏหลาย logical paths
  * หาก path เป็น primary identity ความสัมพันธ์ใน graph, vector index และ provenance อาจขาดเมื่อไฟล์ถูกย้าย
  * Drive letter เช่น `D:` หรือ `G:` สามารถเปลี่ยนได้

* **เหตุผลที่เสนอ**

  * GenesisFS ควรใช้ stable file identity แยกจาก logical path และ physical locator
  * Content ควรอ้างด้วย object identity
  * File version ควรมี identity ของตัวเอง

* **ผลลัพธ์ที่ต้องการ**

  * `file_id` สำหรับ logical file
  * `version_id` สำหรับแต่ละ revision
  * `object_id` สำหรับ content bytes
  * Logical path เป็น namespace projection
  * Physical locator เปลี่ยนได้โดยไม่ทำลาย identity
  * Rename หรือ move แล้ว graph relations, history และ provenance ยังอยู่ครบ

---

## 5. ระบบต้องรู้ประวัติของไฟล์และ State มากกว่าแค่ค่าปัจจุบัน

* **Issue / Concern**

  * Application state, session, mapping และ storage placement เปลี่ยนตามเวลา
  * การเก็บเฉพาะค่าปัจจุบันทำให้ไม่สามารถตอบได้ว่า state เปลี่ยนเมื่อไรหรือใครเป็นผู้เปลี่ยน
  * Agent อาจแก้ข้อมูลจากฐานความจริงที่ล้าสมัย หรือจำข้อมูลที่ถูก supersede ไปแล้ว

* **เหตุผลที่เสนอ**

  * ขยาย bitemporal model ของ GenesisBlockDB ไปยัง file version, application state, mapping, policy และ agent memory
  * แยกเวลาที่ข้อมูลมีผลจริงออกจากเวลาที่ระบบรับรู้หรือบันทึกข้อมูล

* **ผลลัพธ์ที่ต้องการ**

  * ตอบได้ว่าข้อมูลมีผลตั้งแต่เมื่อไร
  * ระบบรับรู้ข้อมูลเมื่อไร
  * ใครหรือ agent ใดเป็นผู้เปลี่ยน
  * เปลี่ยนจาก version ใด
  * State ใดถูก supersede
  * สามารถย้อนดูหรือ restore state ณ ช่วงเวลาหนึ่งได้

---

## 6. จำนวนไฟล์ขนาดเล็กสร้าง Disk I/O และ Metadata Overhead

* **Issue / Concern**

  * Repository สมัยใหม่มีไฟล์จำนวนมากจาก `node_modules`, build cache, package cache, worktree และ agent sandbox
  * เมื่อมีหลาย repository จำนวนไฟล์อาจเพิ่มเป็นหลักแสน
  * HDD ได้รับผลกระทบจาก random seek อย่างรุนแรง
  * แม้ SSD ลด seek time แต่ยังมี syscall, metadata lookup, antivirus, watcher และ indexing overhead
  * การทำ workspace copy หลายชุดสร้างข้อมูลซ้ำจำนวนมาก

* **เหตุผลที่เสนอ**

  * ใช้ content-addressed storage และ object packs ลด duplication และจำนวน physical files
  * Materialize เฉพาะ working set ที่ใช้งาน
  * Dependency หรือ content ที่ไม่เปลี่ยนควรแชร์ object เดียวกันระหว่าง workspace

* **ผลลัพธ์ที่ต้องการ**

  * ลด physical file count
  * ลด duplicate bytes
  * สร้าง workspace ใหม่ได้เร็ว
  * รองรับ copy-on-write workspace
  * Active repository ยังเป็นไฟล์ NTFS ปกติ
  * Cold repository สามารถ compact หรือเก็บใน object representation ได้

---

## 7. JIT ไม่ควร Render ทุกไฟล์ทุกครั้งที่เปิด

* **Issue / Concern**

  * หากทุก file read ต้องผ่าน database query และ render ใหม่ จะเพิ่ม latency และ dependency ต่อ Genesis runtime
  * Editor, compiler และ file watcher อาจอ่านไฟล์จำนวนมาก ทำให้เกิด hydration storm
  * Tool compatibility จะลดลงหากไฟล์ไม่ได้อยู่ในรูปแบบปกติ
  * การเรียก JIT ว่า per-request rendering ทำให้เข้าใจ architecture ผิด

* **เหตุผลที่เสนอ**

  * JIT ควรหมายถึงการ materialize เมื่อจำเป็น แล้ว cache physical representation ไว้ตราบใดที่ยัง active
  * เมื่อ materialize แล้ว application ควรอ่านผ่าน NTFS ตามปกติ
  * Genesis ทำงานเฉพาะ activation, hydration, dirty tracking, commit และ eviction

* **ผลลัพธ์ที่ต้องการ**

  * `INDEXED_ONLY`
  * `HYDRATING`
  * `MATERIALIZED_CLEAN`
  * `MATERIALIZED_DIRTY`
  * `COMMITTING`
  * `EVICTABLE`
  * Cache hit อ่านไฟล์ตรงจาก filesystem
  * Cold data hydrate กลับเมื่อใช้งาน
  * Dirty data ห้าม evict จนกว่าจะ commit หรือ rollback

---

## 8. ต้องรักษาความเข้ากันได้กับ VS Code และ Windows Applications

* **Issue / Concern**

  * Application ปัจจุบันคาดหวัง path, directory และ file semantics แบบ Windows ปกติ
  * VS Code, Git, TypeScript Server, Ollama, Docker และ compiler ไม่ได้ออกแบบให้ query GenesisBlockDB โดยตรง
  * การบังคับให้ทุก application ใช้ custom API จะทำให้ switching cost สูงเกินไป

* **เหตุผลที่เสนอ**

  * Genesis ควร expose data เป็น normal files และ normal paths
  * เริ่มจาก directory materialization, Known Folder redirection และ managed Junction
  * File-level virtualization ควรทำภายหลังเฉพาะ workload ที่เหมาะสม

* **ผลลัพธ์ที่ต้องการ**

  * Application เปิดไฟล์ผ่าน path เดิมได้
  * VS Code, Git และ build tools ทำงานได้โดยไม่ต้องมี plugin บังคับ
  * ผู้ใช้สามารถ export กลับเป็น directory ปกติ
  * รองรับ Junction, ProjFS, CFAPI หรือ WinFsp ตามระดับความจำเป็น
  * ไม่เริ่มจาก kernel filesystem driver หาก user-mode solution ยังเพียงพอ

---

## 9. Junction ช่วยย้าย Path แต่ไม่ช่วยเรื่อง Performance หรือ Lifecycle

* **Issue / Concern**

  * Junction ทำให้ path บน C ชี้ไป D หรือ G ได้
  * แต่ถ้า target เป็น HDD การอ่านยังช้าตาม HDD และอาจ timeout
  * Junction ไม่มี quota, cache, version, integrity, rollback หรือ recovery manifest
  * หลังลง Windows ใหม่ Junction ที่อยู่บน C อาจหาย แม้ content บน D ยังอยู่

* **เหตุผลที่เสนอ**

  * Genesis ต้องใช้ Junction เป็นเพียง compatibility primitive
  * Storage Governor ต้องเป็นผู้ตัดสิน placement และ performance tier
  * Recovery Runtime ต้องสร้าง mapping กลับหลัง reinstall

* **ผลลัพธ์ที่ต้องการ**

  * Managed Junction creation
  * Copy, verification และ rollback
  * Target performance check
  * Pin latency-sensitive data บน SSD
  * SSD hot cache สำหรับ persistent tier ที่ช้ากว่า
  * Junction definitions อยู่ใน Recovery Manifest
  * Restore mapping ได้โดยไม่ต้องจำ command ด้วยตนเอง

---

## 10. Application ติดตั้งบนไดรฟ์อื่น แต่ AppData ยังทำให้ C เต็ม

* **Issue / Concern**

  * ผู้ใช้ติดตั้ง application บน G: แต่ cache, sessions, logs และ package state ยังถูกเขียนใน `%AppData%`
  * ข้อมูลเหล่านี้โตแบบเงียบ ๆ
  * Drive C อาจลดจากพื้นที่ว่างหลายสิบ GB เหลือไม่กี่ร้อย MB
  * เมื่อ C ใกล้เต็ม application อาจ error, update ไม่สำเร็จ หรือเขียน database ไม่สมบูรณ์
  * ผู้ใช้ไม่รู้ว่าอะไรลบได้หรือควรเก็บ

* **เหตุผลที่เสนอ**

  * Genesis Storage Governor ต้อง scan, attribute และ classify storage ต่อ application
  * ต้องแยก cache, session, logs, model, user state, database และ credential
  * ไม่ควรย้าย AppData ทั้งก้อนแบบ generic

* **ผลลัพธ์ที่ต้องการ**

  * แสดงว่า application ใดใช้พื้นที่เท่าไร
  * แสดง growth ต่อวัน
  * ระบุว่าอะไร reclaimable
  * ตั้ง quota และ retention
  * ย้าย session หรือ model ไป volume อื่น
  * เก็บ recent data บน SSD
  * Evict regenerable cache เมื่อ C ใกล้เต็ม
  * ห้ามแตะ credential หรือ machine-bound data โดยไม่มี policy

---

## 11. ต้องมี Application Storage Adapter รายแอป

* **Issue / Concern**

  * แต่ละ application ใช้ directory structure, file locking และ update behavior ต่างกัน
  * บาง directory เป็น cache แต่บาง directoryเป็น database หรือ credential
  * การย้ายแบบ generic อาจทำให้ application เปิดไม่ได้หรือสร้าง directory ใหม่กลับบน C
  * Background service อาจยังถือ file handle อยู่ระหว่าง relocation

* **เหตุผลที่เสนอ**

  * สร้าง adapter manifest ที่บอกว่าแต่ละ path คือข้อมูลชนิดใด
  * ระบุ process/service ที่ต้องหยุด
  * ระบุว่า Junction รองรับหรือไม่
  * กำหนด backup, verification และ rollback ต่อ application

* **ผลลัพธ์ที่ต้องการ**

  * Adapter สำหรับ Ollama, Codex, VS Code, Docker Desktop และ application ที่มี pain สูง
  * Compatibility test ต่อ version
  * Allowlist แทนการ relocate ทุก application
  * Mark adapter incompatible เมื่อทดสอบไม่ผ่าน
  * Community หรือ plugin ecosystem สำหรับเพิ่ม adapter ภายหลัง

---

## 12. ผู้ใช้ไม่ควรต้องบริหาร Drive C ด้วยตนเองตลอดเวลา

* **Issue / Concern**

  * ผู้ใช้มักเก็บไฟล์ไว้ Desktop และ Downloads
  * Cache และ AppData โตโดยไม่แจ้ง
  * ผู้ใช้เริ่มแก้เมื่อพื้นที่ใกล้หมดแล้ว
  * การซื้อ SSD ใหญ่ขึ้นช่วยเพียงเลื่อนปัญหาออกไป
  * Windows ไม่มี application-aware storage policy กลางที่รู้ว่าอะไรสำคัญหรือสร้างใหม่ได้

* **เหตุผลที่เสนอ**

  * Genesis Storage Governor ควรดูแล disk pressure แบบ proactive
  * Known Folders ควรถูก redirect ไป persistent volume อย่างถูกวิธี
  * Storage policy ต้องทำงานตาม threshold

* **ผลลัพธ์ที่ต้องการ**

  * Warning ก่อนพื้นที่วิกฤต
  * Action threshold สำหรับย้ายหรือ evict
  * Emergency reserve
  * Cleanup เฉพาะข้อมูลที่ regenerate ได้
  * Compress logs และ old sessions
  * ห้ามลบ user files หรือ unknown database อัตโนมัติ
  * Dashboard อธิบายได้ว่าพื้นที่ถูกใช้โดยอะไร

---

## 13. Windows และ User Data มี Lifecycle คนละแบบ

* **Issue / Concern**

  * Windows, drivers และ application binaries สามารถติดตั้งใหม่ได้
  * User files, projects, models และ sessions ควรอยู่ต่อ
  * เมื่อทุกอย่างอยู่ C: การลง Windows ใหม่ทำให้ต้อง backup และ restore ข้อมูลจำนวนมาก
  * หากแยกไดรฟ์แบบ manual ผู้ใช้ยังต้องสร้าง Junction และ mapping ใหม่เอง

* **เหตุผลที่เสนอ**

  * แยก OS Plane ออกจาก Persistent State Plane
  * ให้ Genesis เป็น Control Plane ที่จำ mapping และ policy
  * C ควรเป็น OS, runtime และ hot-cache layer
  * D/G ควรเป็น persistent state layer ตาม hardware ของเครื่อง

* **ผลลัพธ์ที่ต้องการ**

  * ลง Windows ใหม่โดย persistent data ยังอยู่
  * Genesis ตรวจพบ persistent volume เดิม
  * Restore Known Folder และ Junction mappings
  * Reattach portable application state
  * แจ้งว่า application ใดต้องติดตั้งใหม่
  * แยกข้อมูลที่ portable ออกจาก machine-bound state

---

## 14. การแยก Partition ไม่ใช่ Backup

* **Issue / Concern**

  * C และ D อาจเป็นคนละ partition แต่ยังอยู่ SSD ลูกเดียวกัน
  * หาก SSD เสีย ทั้งสอง partition หายพร้อมกัน
  * ผู้ใช้อาจเข้าใจผิดว่าการย้ายข้อมูลออกจาก C เพียงพอต่อ disaster recovery

* **เหตุผลที่เสนอ**

  * Genesis ต้องแยก OS reinstall recovery ออกจาก hardware disaster recovery
  * Backup หรือ replica ต้องอยู่คนละ failure domain

* **ผลลัพธ์ที่ต้องการ**

  * Persistent state บน D/G
  * Backup ไป NAS, external drive หรือ cloud
  * Integrity verification
  * Backup status ใน Recovery Manifest
  * แจ้งชัดว่าข้อมูลใดมีเพียง local copy
  * Restore workflow สำหรับกรณี drive failure

---

## 15. Drive Letter ไม่เหมาะเป็น Persistent Storage Identity

* **Issue / Concern**

  * หลังลง Windows ใหม่ D อาจกลายเป็น E หรือ G
  * Junction และ path ที่ hard-code drive letter จะใช้ไม่ได้
  * External drive และ NAS mount อาจเปลี่ยนตำแหน่ง

* **เหตุผลที่เสนอ**

  * Storage location ต้องอ้างด้วย persistent volume identity และ relative path
  * Recovery Runtime ต้องค้นหา volume ก่อนสร้าง mapping

* **ผลลัพธ์ที่ต้องการ**

  * Volume GUID
  * Volume serial หรือ Genesis volume marker
  * Relative storage locator
  * Drive rediscovery
  * Automatic path remapping
  * Report เมื่อ target volume หายหรือไม่ตรง identity

---

## 16. Data รอดจาก Reinstall ไม่ได้แปลว่า Application Restore ได้ทั้งหมด

* **Issue / Concern**

  * Registry, services, scheduled tasks และ package registrations อยู่ใน OS
  * DPAPI, EFS, SID และ hardware-bound licenses อาจไม่ portable
  * การ copy AppData กลับอย่างเดียวอาจไม่ทำให้ application กลับมาสมบูรณ์

* **เหตุผลที่เสนอ**

  * Recovery Runtime ต้องจำแนกระดับการ restore
  * ห้ามสัญญาว่า restore ทุก application ได้โดยไม่มี adapter และ evidence

* **ผลลัพธ์ที่ต้องการ**

  * Portable state
  * Restorable state
  * Reinstall-required state
  * Machine-bound state
  * Credential-bound state
  * รายงานข้อจำกัดก่อน reinstall
  * Backup key/credential เฉพาะเมื่อมี security design ที่เหมาะสม

---

## 17. Cloud และ NAS ไม่ควรเป็น Local Transaction Authority

* **Issue / Concern**

  * Google Drive, NAS หรือ cloud API มี latency, offline state และ conflict model
  * External provider ไม่สามารถ commit atomic พร้อม Genesis WAL ได้โดยตรง
  * ผู้ใช้อาจลบหรือแก้ไฟล์จากภายนอก Genesis
  * การเปิด live SQLite database ผ่าน shared filesystem มีความเสี่ยง

* **เหตุผลที่เสนอ**

  * Local Genesis state ต้องเป็น authority
  * External storage ใช้เป็น replica, backup, sharing, cold tier หรือ hydration source
  * ใช้ Outbox Pattern สำหรับ asynchronous sync

* **ผลลัพธ์ที่ต้องการ**

  * `local_committed`
  * `cloud_pending`
  * `cloud_uploaded`
  * `cloud_verified`
  * `cloud_conflict`
  * Hash verification
  * Retry และ conflict state
  * Remote deletion ไม่ลบ local authority โดยอัตโนมัติ

---

## 18. Agent Memory ไม่ควรเก็บทุกข้อความเป็นความจำถาวร

* **Issue / Concern**

  * Chat history โตต่อเนื่อง
  * Raw conversation มีข้อความซ้ำ, correction, speculation และ orphan context
  * Retrieval อาจดึงข้อมูลเก่าหรือข้อมูลที่ถูกแก้ไขแล้ว
  * Context window บวมและลดคุณภาพการตัดสินใจของ agent

* **เหตุผลที่เสนอ**

  * GenesisBlockDB ควรมี Memory Lifecycle Policy
  * Memory ต้องผ่าน admission, promotion, consolidation และ supersession
  * แยก episodic, semantic และ core memory

* **ผลลัพธ์ที่ต้องการ**

  * Admission policy
  * Promotion/demotion
  * Consolidation
  * Contradiction detection
  * Supersession
  * Forgetting/TTL
  * Pinning
  * Provenance
  * Raw session ไม่ถูกยกระดับเป็น core memory ทั้งหมด

---

## 19. Agent หลายตัวสร้าง Workspace ซ้ำและเสี่ยงชนกัน

* **Issue / Concern**

  * Multi-agent coding อาจสร้าง repo copy หรือ worktree หลายชุด
  * Dependencies และไฟล์ที่ไม่เปลี่ยนถูก duplicate
  * Agent อาจแก้ไฟล์ชุดเดียวกันโดยไม่มี lease หรือ provenance
  * การย้อนกลับและ merge ยากเมื่อไม่มี snapshot boundary

* **เหตุผลที่เสนอ**

  * Genesis ควรรองรับ base workspace และ copy-on-write agent workspace
  * ใช้ shared immutable objects
  * มี workspace lease, dirty tracking และ snapshot

* **ผลลัพธ์ที่ต้องการ**

  * Fast workspace clone
  * Minimal added disk
  * Snapshot/rollback
  * Merge workflow
  * Per-agent provenance
  * Active lease
  * File/object deduplication
  * Human และ agent workspace แยกกันโดยไม่ copy ทุกอย่าง

---

## 20. Agent ต้องได้ Context ตาม Task ไม่ใช่โหลดทุกอย่าง

* **Issue / Concern**

  * การใส่ project context ทั้งหมดลง prompt ทำให้ context window บวม
  * Vector similarity อย่างเดียวอาจดึงข้อมูลที่คล้ายแต่ไม่สำคัญ
  * Decision, risk, test และ dependency มีความสัมพันธ์เชิง graph ที่ similarity ไม่เห็น
  * Context ที่ไม่มี provenance ทำให้ agent ไม่รู้ว่าข้อมูลมาจากไหน

* **เหตุผลที่เสนอ**

  * GenesisBlockDB ควรมี Context Package Compiler
  * ใช้ graph, vector, temporal state, impact และ token budget ร่วมกัน
  * สร้าง package ตาม task แทนการโหลดทั้ง repository

* **ผลลัพธ์ที่ต้องการ**

  * Relevant files
  * Current decisions
  * Open risks
  * Recent changes
  * Related tests
  * Agent memory
  * Token estimate
  * Provenance
  * Coverage/incompleteness status
  * Context package ที่ reproducible และ audit ได้

---

## 21. Semantic Files ควรเป็น Derived Views ไม่ใช่ข้อมูลที่แก้ด้วยมือเสมอไป

* **Issue / Concern**

  * เอกสาร context, decision summary และ impact report ถูกสร้างซ้ำในแต่ละ session
  * ไฟล์อาจล้าสมัยเมื่อ source state เปลี่ยน
  * Agent ไม่รู้ว่า summary สร้างจาก revision ใด

* **เหตุผลที่เสนอ**

  * Genesis JIT ควรรองรับ Semantic Virtual Files
  * Derived file ต้องมี query definition, dependency frontier และ provenance

* **ผลลัพธ์ที่ต้องการ**

  * `current-context.md`
  * `related-decisions.md`
  * `active-risks.md`
  * `impact-graph.json`
  * `session-handoff.md`
  * Regenerate เมื่อ dependency เปลี่ยน
  * Cache เมื่อ source frontier ยังเดิม
  * แสดงว่า output ครอบคลุมหรือขาดข้อมูลส่วนใด

---

## 22. ระบบต้องมี Provenance และ Audit ที่ละเอียดพอสำหรับ Agent

* **Issue / Concern**

  * ในระบบ autonomous agent ไม่เพียงต้องรู้ว่าข้อมูลคืออะไร แต่ต้องรู้ว่าใครหรือ model ใดสร้าง
  * Output อาจมาจาก tool, user, model inference หรือ external source
  * หากไม่มี provenance จะตรวจสอบ error และ rollback ยาก

* **เหตุผลที่เสนอ**

  * Provenance ต้องเป็น first-class state ใน GenesisBlockDB
  * ทุก mutation และ artifact ต้องเชื่อมกับ actor, task, tool และ source

* **ผลลัพธ์ที่ต้องการ**

  * Actor/user/agent identity
  * Model name/version
  * Task and execution ID
  * Tool calls
  * Source references
  * Confidence
  * Parent version
  * Supersession status
  * Audit timeline
  * Reproduce หรือ trace การสร้าง artifact ได้

---

## 23. ไม่ควรขยาย Feature ก่อนพิสูจน์ Core Reliability

* **Issue / Concern**

  * Genesis Block มีขอบเขตกว้างทั้ง database, filesystem, storage manager, agent memory และ recovery
  * หากทำทุก feature พร้อมกันจะเพิ่ม architecture risk และทำให้ validation ไม่ชัด
  * Consumer UI หรือ cloud sync อาจบดบังปัญหา core durability

* **เหตุผลที่เสนอ**

  * พัฒนาตาม dependency order
  * แต่ละ phase ต้องมี exit criteria ที่ตรวจสอบได้

* **ผลลัพธ์ที่ต้องการ**

  * Phase 0: WAL และ crash recovery
  * Phase 1: Managed Object Store
  * Phase 2: File identity และ namespace
  * Phase 3: Windows materialization
  * Phase 4: Storage Governor
  * Phase 5: JIT และ agent workspace
  * Phase 6: Recovery, NAS และ cloud replica
  * Phase 7: Advanced MemoryOS

---

## 24. GenesisBlockDB ไม่ควรแข่งขันด้วยการเป็น Database ทั่วไปอีกตัว

* **Issue / Concern**

  * PostgreSQL, MongoDB และระบบฐานข้อมูลเดิมมี ecosystem, tooling, HA และ maturity สูง
  * การพยายามเลียนแบบ feature breadth จะทำให้ Genesis สูญเสียจุดต่าง
  * ผู้ใช้ไม่มีเหตุผลย้ายฐานข้อมูลทั่วไปเพียงเพราะมี graph หรือ vector เพิ่ม

* **เหตุผลที่เสนอ**

  * GenesisBlockDB ควรโฟกัส transactional state, object identity, temporal history, provenance และ agent memory
  * จุดแข็งคือการรวมหลาย storage semantics ภายใต้ authority เดียว

* **ผลลัพธ์ที่ต้องการ**

  * Category: Local-first State and Memory Kernel
  * Embedded in-process usage
  * AI-native transactions
  * File/object/memory relationships
  * Agent context compiler
  * Application state persistence
  * ไม่ claim ว่าแทน database ทุก workload

---

## 25. จุดขายต้องเป็นผลลัพธ์ของผู้ใช้ ไม่ใช่เทคโนโลยีภายใน

* **Issue / Concern**

  * ผู้ใช้ทั่วไปไม่สนใจว่าใช้ graph, vector, WAL หรือ content hash
  * การขายว่า “รวมไฟล์เป็น `.db`” อาจสร้างความกังวลเรื่อง lock-in และ corruption
  * Junction และ object store ไม่ใช่ value proposition ด้วยตัวเอง

* **เหตุผลที่เสนอ**

  * แปลง technical architecture เป็น user outcome ที่เข้าใจง่าย
  * Consumer และ developer อาจต้องมี product wedge คนละแบบ

* **ผลลัพธ์ที่ต้องการ**

  * Consumer: ป้องกัน C เต็มและรักษาข้อมูลข้าม Windows reinstall
  * Developer: instant, deduplicated, reproducible workspace
  * AI: persistent memory, context package และ agent workspace
  * ข้อมูลสามารถ export กลับเป็นไฟล์ปกติ
  * มี rollback และ recovery ที่พิสูจน์ได้

---

# Summary of the Proposed Direction

* **ปัญหา:** State, files, graph, vector และ memory กระจายอยู่หลายระบบ
  **แนวทาง:** ให้ GenesisBlockDB เป็น authority ของ identity, transaction, version และ provenance

* **ปัญหา:** ไฟล์ใหญ่และไฟล์จำนวนมากไม่เหมาะเก็บใน SQLite ก้อนเดียว
  **แนวทาง:** เพิ่ม Managed Content-addressed Object Store

* **ปัญหา:** Path และ drive letter เปลี่ยนได้
  **แนวทาง:** เพิ่ม Stable File Identity, Version Identity และ Logical Namespace

* **ปัญหา:** แอปต้องการไฟล์ Windows ปกติ
  **แนวทาง:** เพิ่ม GenesisFS และ JIT Materialization โดยใช้ NTFS, Junction และ Known Folder ก่อน

* **ปัญหา:** C เต็มจาก AppData, cache และ session
  **แนวทาง:** เพิ่ม Storage Governor และ Application Storage Adapters

* **ปัญหา:** ลง Windows ใหม่แล้ว mapping หาย
  **แนวทาง:** เพิ่ม Recovery Manifest และ Recovery Runtime

* **ปัญหา:** HDD มี capacity แต่ช้า
  **แนวทาง:** เพิ่ม Hot/Warm/Cold Tiering และ SSD Hot Cache

* **ปัญหา:** Agent memory และ context โตเกินควบคุม
  **แนวทาง:** เพิ่ม Memory Lifecycle และ Context Package Compiler

* **ปัญหา:** Agent workspace ซ้ำและชนกัน
  **แนวทาง:** เพิ่ม Copy-on-write Workspace, Snapshot, Lease และ Provenance

* **ปัญหา:** Product scope กว้างเกินไป
  **แนวทาง:** พัฒนาตามลำดับ Core Reliability → Object Store → Namespace → Windows Runtime → Storage Governor → MemoryOS
