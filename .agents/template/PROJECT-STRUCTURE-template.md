---
title: "PROJECT STRUCTURE: <Project Name>"
doc_id: "PROJECT-STRUCTURE-<slug>"
status: "draft"
version: "0.2.0"
updated: "YYYY-MM-DD"
owner: "<owner>"
source_of_truth: true
related_docs: []
---

# PROJECT STRUCTURE: <Project Name>

## 1. Purpose
Describe what this repository/workspace contains and how humans and agents should navigate it.

## 2. Source of Truth
- PRD:
- SRS:
- SDD:
- C4:
- Execution Governance:
- Feature Index:
- Design System:

## 3. Workspace Layout
```text
<repo>/
+-- src/
+-- public/
+-- comp/
+-- docs/
+-- .agents/
+-- scripts/
+-- workflows/
```

## 4. Directory Contract
| Path | Owner | Purpose | Allowed Content | Not Allowed |
|---|---|---|---|---|

## 5. Documentation Layout
```text
docs/
+-- PRD-*.md
+-- SRS-*.md
+-- SDD-*.md
+-- architecture/
+-- features/
+-- api/
+-- adr/
+-- design/
+-- runbooks/
+-- test-plans/
```

## 6. Feature Folder Mapping
| PRD System | Folder |
|---|---|
| SYSTEM-01 Mission Control Experience | docs/features/mission-control/ |
| SYSTEM-02 Project Roadmap Management | docs/features/project-roadmap/ |
| SYSTEM-03 Docs to Code | docs/features/docs-to-code/ |
| SYSTEM-04 Diagram to Doc | docs/features/diagram-to-doc/ |
| SYSTEM-05 Agent Team Management | docs/features/agent-team/ |
| SYSTEM-06 Integration Bridge | docs/features/integration-bridge/ |
| SYSTEM-07 Governance Access Control | docs/features/governance-access/ |
| SYSTEM-08 Genesis Knowledge System | docs/features/genesis-knowledge-system/ |
| SYSTEM-09 Traceability Audit Verification | docs/features/traceability-audit/ |
| SYSTEM-10 Execution Governance | docs/features/execution-governance/ |

## 7. Agent Workspace Layout
```text
.agents/
+-- auditor/
+-- doc_writer/
+-- pm/
+-- qa/
```

## 8. Build and Test Entry Points
| Command | Purpose | Owner |
|---|---|---|

## 9. Naming Conventions
- Docs:
- Features:
- Branches:
- Components:
- APIs:

## 10. Boundaries
- Frontend:
- Backend/integration:
- Docs:
- Generated artifacts:
- Agent/private workspace:

## 11. Cleanup Rules
- What may be removed:
- What must be archived:
- What must never be deleted without owner approval:

## 12. Open Questions
-

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-15 | Initial template scaffold aligned with document versioning governance. |

