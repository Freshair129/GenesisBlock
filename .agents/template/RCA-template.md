---
title: "RCA: <Incident or Problem Name>"
doc_id: "RCA-<slug>"
status: "draft"
version: "0.1.0"
updated: "YYYY-MM-DD"
owner: "<owner>"
source_of_truth: true
related_docs: []
---

# RCA: <Incident or Problem Name>

## 1. Executive Summary
What happened, who was affected, and what was fixed.

## 2. Impact
- User impact:
- Business impact:
- System impact:

## 3. Timeline
| Time | Event |
|---|---|
|  |  |

## 4. Root Cause
State the confirmed root cause with evidence.

## 5. Why It Escaped
- Missing test:
- Missing guardrail:
- Missing review step:

## 6. Corrective Actions
| Action | Owner | Status | Target Date |
|---|---|---|---|
|  |  |  |  |

## 7. Prevention
### Universal Verification & Prevention SOP
- [ ] **Step 1: Root Cause Validation** - Confirm the root cause is identified with empirical evidence (e.g., failed test, logs).
- [ ] **Step 2: Corrective Action Implementation** - Implement the fix strictly targeting the root cause without side effects.
- [ ] **Step 3: Multi-Layer Verification** - Verify the fix through automated tests, manual checks, and regression testing.
- [ ] **Step 4: Context & Memory Update** - Update relevant documentation (RCA, ADR, SDD) and `MEMORY.md` to ensure the fix is documented and persistent.
- [ ] **Step 5: Process Guardrail** - Implement a new rule, test, or automation to prevent the exact same issue from recurring.

### Specific Prevention Measures
- Code:
- Tests:
- Process:

## 8. Related Documents
-

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-15 | Initial template scaffold aligned with document versioning governance. |

