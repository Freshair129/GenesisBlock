## R10 — Complexity-Based Execution Path

Select the minimum process that safely satisfies the request.

Avoid under-engineering.
Avoid over-engineering.

---

### C-1 — Direct Implementation

Workflow:

```text
Text
→ Code
```

Use when:

* Small isolated task
* Single-file modification
* Bug fix with obvious root cause
* Script or utility
* Low-risk change

Examples:

* Fix typo
* Add simple validation
* Update configuration
* Small helper function

Requirements:

* Scope verification
* Basic validation

Documentation:

Not required beyond inline explanation.

---

### C-2 — Documentation-Driven Implementation

Workflow:

```text
Text
→ Doc
→ Code
```

Use when:

* New feature
* Multi-file modification
* Non-trivial business logic
* Public API change
* Medium-risk change

Examples:

* Authentication flow
* New endpoint
* Payment integration
* Feature enhancement

Requirements:

* Feature Spec
* RCA (for bug fixes)
* Impact Analysis

Documentation must be approved before implementation.

---

### C-3 — Architecture-Driven Implementation

Workflow:

```text
Text
→ Doc
→ Diagram
→ Code
```

Use when:

* Architecture change
* Distributed systems
* Multiple services
* Data flow redesign
* High-risk change
* Platform-level feature

Examples:

* Microservice introduction
* Event-driven architecture
* Database migration
* Multi-system integration
* Workflow orchestration

Required Artifacts:

1. Specification
2. Architecture Diagram
3. Sequence Diagram (if applicable)
4. API Contract (if applicable)
5. Migration Plan (if applicable)

All artifacts must be reviewed before implementation.

---

### Escalation Rule

If uncertainty increases during execution:

```text
C-1 → C-2
C-2 → C-3
```

Never downgrade complexity after approval without justification.

---

### Selection Rule

Always choose the lowest complexity level that:

1. Maintains correctness
2. Maintains safety
3. Preserves maintainability

When uncertain:

Choose the higher level.

---

### Verification Requirements

| Level | Verification                                       |
| ----- | -------------------------------------------------- |
| C-1   | Validation                                         |
| C-2   | Tests + Documentation Review                       |
| C-3   | Tests + Documentation Review + Architecture Review |

---

### Examples

Fix typo

```text
C-1
Text → Code
```

Add login feature

```text
C-2
Text → Doc → Code
```

Split monolith into services

```text
C-3
Text → Doc → Diagram → Code
```
