---
doc_id: "AGENT-TEMPLATE"
version: "0.1.0"
created_at: "YYYY-MM-DDTHH:mm:ss+07:00,<owner>"
last_update: "YYYY-MM-DDTHH:mm:ss+07:00,<owner>"
status: "draft"
attributes:
  domain: "<domain>"
  scope: "<path or workspace>"
  doc_type: "agent-operating-contract"
---

# AGENTS.md

## Scope
This instruction applies to:

```text
<path-or-workspace>
```

## Agent Operating Contract
- Role:
- Allowed actions:
- Forbidden actions:
- Escalation rules:

## Source of Truth
- Product SSOT:
- Requirements SSOT:
- Architecture view:
- System design:
- Execution governance:
- Feature index:
- Design system:

## Documentation Rules
- Use human-first SWE documents.
- Use templates from `.agents/doc_writer/template/`.
- Place feature specs under the correct `docs/features/<system-folder>/`.
- Keep atoms as derived artifacts, not required human authoring format.

## Engineering Rules
- Keep changes surgical.
- Preserve existing architecture boundaries.
- Do not reintroduce deprecated runtime patterns.
- Follow local code style and test conventions.

## Git Rules
- Stage only task-relevant files.
- Do not include unrelated dirty worktree changes.
- Use clear commit messages.
- Use `git mv` for file reorganizations.

## Verification Rules
- Declare what was tested.
- Report tests that were not run.
- Attach visual evidence for UI changes.
- Include policy/audit evidence for RBAC/ABAC or MCP changes.

## Risk Rules
- LOW:
- MEDIUM:
- HIGH:
- Escalate when:

## Local Exceptions
Document any local rule that differs from the root instruction.

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | YYYY-MM-DD | Initial agent operating contract template with canonical doc_id metadata. |



