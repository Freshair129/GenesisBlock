---
proposed_id: TDD--DOCUMENTATION-GOVERNANCE-SSOT-ENFORCEMENT
type: tdd
status: candidate
version: 0.1.0b
created_at: 2026-06-13T21:21:44+07:00,ATHER,9b1ced3
last_update: 2026-06-13T21:21:44+07:00,ATHER
attributes:
  domain: documentation-governance
  scope: repository
  language: th
  complexity: C-2
  risk: MEDIUM
---

# TDD--DOCUMENTATION-GOVERNANCE-SSOT-ENFORCEMENT

## 1. Context

GenesisDB ใช้แนวทาง Documentation-Driven Development (DDD) และประกาศให้
`docs/MASTER-SPEC--GENESIS-DB.md` เป็นเอกสารอ้างอิงหลักของระบบ แต่สถานะปัจจุบันยังเป็นการบังคับใช้เชิงมนุษย์มากกว่าเชิงกลไก:

- `ARCHITECTURE.md` ระบุว่า master spec เป็น authoritative technical spec
- `CONTRIBUTING.md` ระบุว่าการเปลี่ยน core behavior ต้อง update master spec
- `AGENT.md` ระบุ workflow ของ agent, RCA, complexity, Definition of Done และ doc-first
- repository ยังไม่มี script, CI workflow, หรือ active git hook ที่ตรวจ doc/code drift โดยตรง

เอกสารนี้ออกแบบชั้น governance enforcement เพื่อทำให้ SSOT, doc diff, agent context diff, และ quality gates ตรวจได้ซ้ำและบังคับใช้ได้จริง
โดยยังไม่เปลี่ยน runtime behavior ของ GenesisDB core engine

## 2. Problem Statement

ปัญหาหลักคือมี SSOT แบบประกาศไว้ แต่ยังไม่มี enforcement ที่เข้มพอ:

1. Code/API/SDK สามารถเปลี่ยนได้โดยไม่มี doc diff ที่เกี่ยวข้อง
2. เอกสารบางไฟล์มี status หรือ checklist ที่ stale เมื่อเทียบกับ code
3. Agent context มี lifecycle แต่ยังไม่ถูก validate ว่า active/candidate/stable ใช้ถูกสถานะ
4. Deprecated specs ยังอยู่ร่วมกับ current specs โดยไม่มี machine-readable replacement chain
5. Test command, dashboard audit, และ API contract สามารถ drift จากเอกสารได้โดย CI ไม่จับ
6. RCA rule ระบุให้บันทึกใน `.brain/rca/` แต่ยังไม่มีตัวตรวจว่าบั๊กฟิกซ์มี RCA จริง

ผลลัพธ์คือ roadmap และ docs อาจให้ภาพความพร้อมที่สูงกว่าหลักฐานจริง และ agent รุ่นถัดไปอาจใช้ context ที่ไม่ใช่ source of truth

## 3. Scope

### 3.1 In Scope

- นิยาม SSOT hierarchy ของ repository
- นิยาม manifest สำหรับ mapping ระหว่าง code area, docs, specs, tests และ owner context
- ออกแบบ governance validator ที่รันได้ใน local และ CI
- ออกแบบ output report ทั้ง JSON และ Markdown
- ออกแบบ quality gates สำหรับ doc diff, agent context diff, changelog, version bump, deprecated docs, RCA, และ API contract drift
- ออกแบบ adoption plan แบบ incremental เพื่อไม่ block งานทันทีเกินไป

### 3.2 Out of Scope

- ไม่แก้ GenesisDB runtime behavior
- ไม่แก้ Rust/Python/Go/Node SDK contract ในเอกสารนี้
- ไม่แก้ dashboard lint หรือ e2e flow ในเอกสารนี้
- ไม่เปลี่ยน release process ภายนอก GitHub Actions
- ไม่เพิ่ม auto-format หรือ mass rewrite เอกสารเดิม

## 4. Technical Solution

### 4.1 SSOT Hierarchy

Repository ควรมี source of truth เป็นลำดับชั้นดังนี้:

| Layer | SSOT | Responsibility |
|---|---|---|
| Product / architecture | `docs/MASTER-SPEC--GENESIS-DB.md` | capabilities, architecture, public behavior |
| Architecture decisions | `docs/adr/ADR--*.md` | decision rationale and trade-offs |
| Feature / module contracts | `docs/SPEC--*.md`, `docs/TDD--*.md` | feature requirements, design, DoD |
| API contracts | `docs/API_REFERENCE.md`, SDK docs | REST, N-API, MCP, SDK payload shapes |
| Agent governance | `AGENT.md` / `AGENTS.md` | agent workflow, DDD/RCA rules, context lifecycle |
| Evidence | tests, audits, `.brain/rca/` | verification and root-cause records |

### 4.2 Governance Manifest

เพิ่ม manifest แบบ machine-readable เช่น:

```text
docs/governance/ssot.manifest.json
```

Manifest ต้อง map code paths กับเอกสารที่เกี่ยวข้อง:

```json
{
  "version": "0.1.0b",
  "areas": [
    {
      "id": "rust-core",
      "paths": ["src/**/*.rs"],
      "required_docs": [
        "docs/MASTER-SPEC--GENESIS-DB.md",
        "docs/API_REFERENCE.md"
      ],
      "required_tests": ["cargo test -- --list"],
      "requires_rca_for_bugfix": true
    },
    {
      "id": "sdk-contracts",
      "paths": ["genesisdb-python/**", "genesisdb-go/**", "index.d.ts"],
      "required_docs": ["docs/API_REFERENCE.md"],
      "contract_checks": ["hql-request-shape"]
    }
  ]
}
```

### 4.3 Validator

เพิ่ม validator เช่น:

```text
scripts/validate-governance.mjs
```

หน้าที่หลัก:

1. Read manifest
2. Inspect git diff หรือ full tree
3. Validate metadata/frontmatter/status/version/changelog
4. Validate doc/code/test relationship ตาม manifest
5. Emit machine-readable JSON report
6. Emit Markdown summary สำหรับ agent และ CI logs
7. Exit non-zero เฉพาะ rule ที่อยู่ใน blocking mode

### 4.4 Rule Set

| Rule ID | Rule | Initial Mode |
|---|---|---|
| GOV-001 | ต้องมี master spec เพียงไฟล์เดียวที่เป็น authoritative spec | blocking |
| GOV-002 | เอกสาร governance ต้องมี frontmatter/status/version | warning |
| GOV-003 | deprecated/superseded docs ต้องมี `superseded_by` | warning |
| GOV-004 | code diff ใน mapped area ต้องมี doc diff หรือ explicit waiver | warning |
| GOV-005 | agent context diff ต้อง update changelog/version | blocking |
| GOV-006 | roadmap ห้าม claim complete ถ้า linked DoD ยังเปิด | warning |
| GOV-007 | public API/SDK contract ต้องตรงกับ docs หรือมี known-drift entry | warning |
| GOV-008 | bugfix diff ต้องมี RCA record ใน `.brain/rca/` หรือ waiver | warning |
| GOV-009 | root test scripts ต้อง resolve และรันได้ตามเอกสาร | warning |
| GOV-010 | audit docs ต้องอ้าง command/URL/test target ที่ตรงกับ test files | warning |

### 4.5 Waiver Model

เพื่อไม่ให้ adoption หนักเกินไป validator ควรรองรับ waiver ที่ traceable:

```yaml
governance_waiver:
  rule: GOV-004
  reason: "documentation-only follow-up approved separately"
  expires_at: 2026-06-20
  approved_by: "Boss"
```

Waiver ที่หมดอายุควรถูก treat เป็น failure

## 5. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Enforcement block งานเดิมมากเกินไป | dev velocity ลด | เริ่มด้วย warning mode แล้วค่อย promote เป็น blocking |
| False positive จาก manifest ที่ยังไม่ครบ | agent เสียเวลาตามแก้ noise | เพิ่ม waiver และรายงาน confidence |
| เอกสาร legacy ไม่ผ่าน rule จำนวนมาก | migration หนัก | ทำ baseline report ก่อน แล้ว enforce เฉพาะ new diffs |
| API contract checker ผูกกับ implementation มากเกินไป | validator เปราะ | เริ่มจาก pattern checks ที่ชัด เช่น HQL request body shape |

## 6. Implementation Plan

### Phase 1: Baseline Documentation

1. เพิ่ม TDD ฉบับนี้
2. Review และ approve scope/rules
3. ระบุรายการ SSOT และ known drift ปัจจุบัน

### Phase 2: Non-blocking Validator

1. เพิ่ม `docs/governance/ssot.manifest.json`
2. เพิ่ม `scripts/validate-governance.mjs`
3. เพิ่ม npm script เช่น `governance:check`
4. รันแบบ full-tree และ diff-aware
5. Generate `docs/AUDIT--DOCUMENTATION-GOVERNANCE.md`

### Phase 3: CI Gate

1. เพิ่ม `.github/workflows/governance.yml`
2. เริ่มจาก warning summary ใน PR
3. Promote `GOV-001` และ `GOV-005` เป็น blocking
4. Promote rule อื่นหลังลด legacy drift แล้ว

### Phase 4: Strict Mode

1. Enforce doc diff for mapped code areas
2. Enforce RCA for bugfixes
3. Enforce deprecated/superseded chains
4. Enforce API contract checks for REST, MCP, N-API, Python SDK, Go SDK

## 7. Testing Strategy

Validator ต้องมี test cases อย่างน้อย:

- pass เมื่อ manifest valid และ docs มี metadata ครบ
- fail เมื่อ master spec หายหรือมีมากกว่าหนึ่ง authoritative spec
- warn เมื่อ code diff ไม่มี doc diff
- fail เมื่อ `AGENT.md` เปลี่ยนแต่ไม่มี changelog/version bump
- warn เมื่อ deprecated doc ไม่มี `superseded_by`
- warn เมื่อ SDK request shape drift จาก API reference
- fail เมื่อ waiver หมดอายุ

Command เป้าหมาย:

```text
npm run governance:check
node --test __test__/governance/*.mjs
```

## 8. Monitoring & Observability

Governance validator ควรออกผลลัพธ์เป็น:

- Console summary สำหรับ local dev
- JSON report เช่น `target/governance/report.json`
- Markdown report เช่น `target/governance/report.md`
- CI annotations สำหรับ blocking findings

Metrics ที่ควรเก็บ:

- จำนวน warning/failure ต่อ rule
- จำนวน known drift ที่ยังไม่ปิด
- จำนวน waiver ที่ active/expired
- จำนวน docs ที่ไม่มี metadata
- จำนวน mapped code areas ที่ไม่มี doc owner

## 9. Rollback Plan

Rollback ต้องทำได้โดยไม่กระทบ runtime:

1. ปิด CI blocking mode กลับเป็น warning
2. revert เฉพาะ workflow/script/manifest ที่เพิ่มใน Phase 2-3
3. เก็บ audit report ไว้เป็น evidence
4. ถ้า rule เปราะ ให้ downgrade เฉพาะ rule นั้นผ่าน manifest config ไม่ต้องลบทั้งระบบ

## 10. Acceptance Criteria

- SSOT hierarchy ถูกระบุแบบ machine-readable
- Validator รันได้ใน local โดยไม่ต้องใช้ network
- Validator แยก warning/blocking ได้
- Agent context changes ต้องถูกตรวจ version/changelog
- Governance report ชี้ doc/code drift ที่พบได้
- CI สามารถเปิดใช้แบบ warning ก่อน strict mode

## 11. Definition of Done

- TDD นี้ถูก review และ approved
- มี manifest สำหรับ doc/code ownership
- มี validator script พร้อม tests
- มี npm script หรือ equivalent command สำหรับรัน validator
- มี CI workflow หรือ documented local gate
- มี baseline audit report สำหรับ drift ปัจจุบัน
- ไม่มี runtime behavior change ของ GenesisDB core engine จาก governance work

## 12. Open Questions

1. Repository ควรใช้ `AGENT.md`, `AGENTS.md`, หรือทั้งสองไฟล์เป็น agent context SSOT
2. จะเริ่ม strict mode ที่ rule ใดก่อน นอกจาก `GOV-001` และ `GOV-005`
3. Known drift ปัจจุบันควรถูกบันทึกเป็น waiver ชั่วคราว หรือเปิดเป็น audit findings
4. RCA path ควรใช้ `.brain/rca/` ตาม directive เดิม หรือย้ายไป `docs/rca/` เพื่อ version control ง่ายขึ้น

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---------|------|--------|---------|-------------|-------|
| 0.1.0b | 2026-06-13 | candidate | Initial TDD for documentation governance and SSOT enforcement. | 9b1ced3 | ATHER |

---

**Please review and approve this documentation. I will generate the validator and enforcement files once approved.**
