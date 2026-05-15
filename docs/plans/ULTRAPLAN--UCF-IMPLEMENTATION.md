# ULTRAPLAN — Universal Context Framework (UCF) Implementation

> **Executor:** Gemini CLI (`gemini --approval-mode yolo`)
> **Reviewer:** Claude / human
> **Branch base:** `gemini/ucf-implementation-plan`
> **Status:** DRAFT (Wait for human sign-off before execution)
> **Last updated:** 2026-05-16T00:30:00+07:00

---

## 0. Context (The Strategic Goal)

The **Universal Context Framework (UCF)** is a transport-agnostic system for **identity-aware**, **policy-controlled**, and **graded-resolution** retrieval. It solves four core problems:
1. **Identity:** "With whose authority?"
2. **Sensitivity:** Protecting PII/PHI/IP.
3. **Scope:** Subagent isolation (POLA).
4. **Cost:** Resolution gradient retrieval (load less, load smarter).

This plan materializes the 5 UCF FEATs defined in `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md`.

---

## 1. Prerequisites & Atom Promotion

Before implementation begins, we must promote the 5 UCF FEATs from `draft` to `active` (or `stable`) and verify their dependencies.

### 1.1 FEAT Atoms to Promote
- `FEAT--POLICY-DECISION-POINT`
- `FEAT--VAULT-COMPOSITION`
- `FEAT--RESOLUTION-EXPAND-ON-DEMAND`
- `FEAT--SUBAGENT-SCOPE-FILTERING`
- `FEAT--STEP-UP-AUTH-PIN`

### 1.2 Required ADRs/CONCEPTs
Ensure the following are `status: stable/active`:
- `FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK`
- `CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT`
- `CONCEPT--ATTRIBUTE-BAG-MODEL`
- `CONCEPT--NAMESPACE-VAULT-BRAIN`
- `CONCEPT--RESOLUTION-GRADIENT`
- `CONCEPT--ABAC-POLICY-ENGINE`
- `ADR--POLICY-AS-DATA-NOT-CODE`
- `ADR--BRING-YOUR-OWN-ATTRIBUTES`
- `ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT`
- `ADR--RESOLUTION-TIER-COUNT`

---

## 2. Implementation Phases

### PHASE 0 — Propagation Plumbing (LOW RISK)

**Goal:** Thread the UCF 4-tuple (`Subject`, `Resource`, `Action`, `Context`) through the retrieval and codegen pipeline.

#### 0.1 Define Core Types
- File: `packages/msp/src/policy/types.ts` (NEW)
- Define `Subject`, `Resource`, `Action`, `RequestContext`, `Decision`, `AttributeBag`.

#### 0.2 Update Retrieval Facade
- Update `packages/msp/src/cognitive/layer.ts` (or equivalent) to accept `Subject` and `RequestContext` in `recall()`, `retain()`, etc.
- Default to a "System" subject if none provided.

#### 0.3 Update Orchestrator / Composer
- Update `packages/msp/src/orchestrator/` to propagate these types down to the GKS storage layer.

#### 0.4 Acceptance
- `npm run typecheck` passes.
- All entry points log the 4-tuple (verify via test logs).
- No behavioral changes.

---

### PHASE 1 — Policy Decision Point (PDP) & Shadow Mode (MEDIUM RISK)

**Goal:** Implement the YAML-based policy engine and run it in "log-only" mode.

#### 1.1 Implementation of `evaluatePolicy`
- File: `packages/msp/src/policy/pdp.ts` (NEW)
- Implement a minimal YAML policy evaluator (~300 LOC) supporting basic operators (equality, membership, set ops).
- Support hot-reloading of policies from `msp/rules/*.yaml`.

#### 1.2 Shadow PEP (Policy Enforcement Point)
- Integrate PDP into `layer.runTask()` and `layer.recall()`.
- **Mode:** `shadow-log`. The PDP runs, but the decision is only logged; the action always proceeds.

#### 1.3 Baseline Policies
- Author `msp/rules/default-permit.yaml` (permits everything).
- Author `msp/rules/subagent-scoping.yaml` (shadow mode: would deny out-of-scope).

#### 1.4 Acceptance
- Decision logs show reasoning traces (which rules matched).
- Hot-reload confirmed: changing a YAML rule reflects in logs immediately.

---

### PHASE 2 — Subagent Scope Filtering (ENFORCED)

**Goal:** Enforce task-level isolation at the composer.

#### 2.1 Task Scope Schema
- Update `packages/msp/src/codegen/load-task.ts` to parse `scope.needs` and `scope.excludes`.

#### 2.2 Enforced PEP at Composer
- Flip `subagent-scoping.yaml` to enforced mode.
- Filter atoms entering subagent context based on `domain` attributes.

#### 2.3 Escalation Pattern
- Implement `layer.escalate()` for subagents to request scope extensions when a PDP deny is encountered.

#### 2.4 Acceptance
- Subagents only see atoms within their declared scope.
- Escalation logs show subagent requests for more context.

---

### PHASE 3 — Vaults & Resolution Gradient (MVP: 2-Tier)

**Goal:** Implement multi-namespace views and `FULL` + `MENTION` retrieval.

#### 3.1 Vault Configuration
- File: `packages/msp/src/orchestrator/vaults.ts` (NEW)
- Support mounting multiple Namespaces into a single logical `Vault`.

#### 3.2 Resolution Tiering (Layer 4 & 5)
- Update composer to assign `FULL` vs `MENTION` tiers based on relevance score and budget.
- Implement `expand()` tool to promote a `MENTION` to `FULL`.

#### 3.3 Acceptance
- Token usage on large tasks drops significantly (60%+).
- Subagents call `expand()` successfully to get details on demand.

---

### PHASE 4 — Step-up Auth & User ABAC (HIGH RISK)

**Goal:** Finalize identity-bound retrieval and defense-in-depth.

#### 4.1 Step-up Auth (PIN)
- Implement a minimal PIN-based step-up provider for local use.
- Integrate into PDP: certain actions (delete, read PHI) require `last_step_up_at < 5m`.

#### 4.2 User Attribute Integration
- Populate `Subject` with real user attributes (roles, clearance) from the environment/session.

#### 4.3 Acceptance
- Attempting to read "confidential" atoms triggers a step-up request.
- Different users see different filtered results for the same query.

---

## 3. Verification Protocol

### Every Phase
1. `npm run typecheck`
2. `npm test`
3. `npm run msp:validate`
4. Manual audit of `trace_id` and `reasoning` logs.

### Phase 2+ (Quality A/B)
- Compare task success rate with and without scope filtering.

---

## 4. STOP & SIGN-OFF
**HALT.** Do not write code for Phase 0 until this plan is approved by a human or Claude (T3).
