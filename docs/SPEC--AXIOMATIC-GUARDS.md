# Functional Specification: Axiomatic Guards (Data Governance)

## 1. Objective
Implement a robust data governance layer called **Axiomatic Guards**. This system enforces integrity constraints based on data "Tiers" to ensure that the engine's core reasoning and specifications (the "Axioms") cannot be corrupted or accidentally overwritten by external agents or plugins.

## 2. Governance Tiers
Every node and edge in GenesisDB will belong to a specific governance tier, defined by its `labels` or `props`.

| Tier | Priority | Governance Rule |
| :--- | :--- | :--- |
| **MASTER** | 0 (Highest) | Read-only for all external FFI/API calls. Can only be mutated by internal consensus logic or system-signed events. |
| **SPEC** | 1 | Requires mandatory `valid_from` and `recorded_at` fields. Mutation triggers a versioned retraction of the old state. |
| **ADR** | 2 | Architecture Decision Records. Requires link to a SPEC node. |
| **USER** | 3 (Lowest) | Default tier. Flexible schema, open mutation. |

## 3. Implementation: The Guard Trait
We will introduce a `Guard` system that intercepts all mutations (`add_node`, `add_edge`, `retract_edge`).

### 3.1 Logic Flow
1.  **Intercept:** When `add_node` is called, the engine checks the `labels`.
2.  **Validate:** If labels contain `"MASTER"`, the engine checks the caller's authorization context.
3.  **Reject:** If unauthorized, return `Error::from_reason("403 Forbidden: MASTER tier is immutable")`.
4.  **Enforce SPEC rules:** If labels contain `"SPEC"`, ensure `props` contain the required temporal metadata.

## 4. Proposed Changes

### 4.1 src/lib.rs
- Add `Tier` enum.
- Update `add_node` and `add_edge` to include a `validate_governance` step.
- Implement a `Guard` internal helper to centralize the rules.

### 4.2 Error Handling
- Introduce `NapiError` mappings for governance violations.

## 5. Implementation Roadmap
1.  **Step 1:** Define the `Tier` logic and the `Guard` interceptor.
2.  **Step 2:** Integrate the guard into `add_node` and `add_edge`.
3.  **Step 3:** Add unit tests to verify that MASTER nodes cannot be overwritten by standard API calls.

Please review and approve this Axiomatic Guards Specification. I will proceed with implementation once approved.
