# UNIVERSAL CONTEXT FRAMEWORK — Specification

> **Status:** `draft` · **Version:** `0.1.1` · **Created:** 2026-05-13 · **Owner:** MSP team
>
> A universal, transport-agnostic framework for **identity-aware**, **policy-controlled**, **graded-resolution** retrieval and context delivery in agentic AI systems. Designed to work for any data domain (medical, finance, legal, code, HR, multi-tenant SaaS) without hardcoding domain knowledge into the core.
>
> This spec describes the **why**, **what**, and **how**. Atoms (`gks/<type>/*.md`) decompose this document into traceable units (CONCEPT / ADR / FEAT / BLUEPRINT / AUDIT).

---

## Table of Contents

- [Abstract](#abstract)
- [§0 Resolved Decisions](#0-resolved-decisions)
- [§1 Motivation & Background](#1-motivation--background)
- [§2 Conceptual Model — Three Axes](#2-conceptual-model--three-axes)
- [§3 Subject-Resource-Action-Context Model](#3-subject-resource-action-context-model)
- [§4 Attribute Bag — Bring-Your-Own-Schema](#4-attribute-bag--bring-your-own-schema)
- [§5 Namespace, Vault, Brain — Terminology and Layering](#5-namespace-vault-brain--terminology-and-layering)
- [§6 Resolution Gradient Retrieval](#6-resolution-gradient-retrieval)
- [§7 ABAC Policy Engine](#7-abac-policy-engine)
- [§8 Step-up Authentication](#8-step-up-authentication)
- [§9 Subagent Context Scoping](#9-subagent-context-scoping)
- [§10 Composition Pipeline — Five Layers](#10-composition-pipeline--five-layers)
- [§11 Phased Implementation](#11-phased-implementation)
- [§12 Threat Model & Landmines](#12-threat-model--landmines)
- [§13 Glossary](#13-glossary)
- [§14 Open Questions](#14-open-questions)
- [§15 References & Crosslinks](#15-references--crosslinks)

---

## Abstract

Modern agentic systems face four converging context problems:

1. **Identity** — multiple users share one agent; the agent must answer "with whose authority?"
2. **Sensitivity** — knowledge bases hold mixed-sensitivity data (PII, PHI, IP, secrets); agents must not leak across trust boundaries.
3. **Scope** — subagents fixing a login bug do not need patient records; loading them pollutes context and increases blast radius for prompt injection.
4. **Cost** — large knowledge graphs cannot all fit in a context window; naive top-K retrieval wastes tokens on tangentially related material.

This spec defines a **single composable framework** that solves all four:

- **Subject / Resource / Action / Context** as the universal request shape.
- **Bring-Your-Own attribute bag** so domain-specific data (HIPAA, PCI, internal-only) lives in policy, not core code.
- **Namespace + Vault + Brain** as three distinct layers (security partition, UX composition, holistic agent identity).
- **Resolution gradient** (`FULL` / `SUMMARY` / `SKELETON` / `MENTION`) so context loads with depth-of-field instead of binary on/off.
- **ABAC Policy Decision Point (PDP)** as a transport-agnostic decision authority, enforced at every retrieval and tool-call entry point.
- **Step-up authentication** for sensitive operations even when the outer session is already authenticated.

The core is **domain-agnostic**. Domain knowledge enters through plugin classifiers (e.g. PII regex, HIPAA tags) and policy files (YAML / Cedar / OPA). Shipping a vertical (e.g. medical agent) is a matter of dropping in a policy pack — no core changes.

---

## §0 Resolved Decisions

The following design choices have been agreed and remove their counterparts from §14 Open Questions. Subsequent sections that depend on them have been updated inline.

| # | Topic | Decision | Date | Notes |
|---|---|---|---|---|
| **D-1** | Policy language v1 | **YAML + minimal built-in operators** (~200 LOC parser/evaluator). | 2026-05-13 | Migration path to Cedar is mechanical translation once policy count exceeds ~30 rules or recursion is needed. |
| **D-2** | Resolution tier count (MVP) | **2-tier: `FULL` + `MENTION`**, plus `expand()` on demand. | 2026-05-13 | 4-tier (adding `SUMMARY`, `SKELETON`) deferred to Phase 4 — only if `expand()` usage telemetry proves graded retrieval is high-value. |
| **D-7** | Default policy posture | **`default-permit` + shadow log in Phase 1**, tighten to `default-deny` per-endpoint from Phase 3 onward. | 2026-05-13 | Prevents breakage of existing flows; gives operators a "what would have been denied" report before enforcement. |
| **D-8** | AttributeBag storage | **Atom metadata (JSON column / frontmatter `attributes:`)**. `Namespace` stays the 4-field composite key in GKS, untouched. | 2026-05-13 | Preserves GKS invariants (Namespace = immutable partition key); pgvector filters use `attributes->>'<key>'`. |

Open questions remaining: see §14.

---

## §1 Motivation & Background

### 1.1 The shared-agent problem

When an agent serves more than one person — whether on a team, in a household, or as a hosted SaaS — it inherits a problem from filesystems and databases: **whose data is whose, and who can see what?**

Today's typical agent solutions:

| Approach | Problem |
|---|---|
| **One knowledge base per user** | No collaboration; reinvented knowledge; sync hell. |
| **One shared base, identical view** | Anyone with session access sees everything. |
| **Partition by hand-curated topics** | Brittle; topics drift; doesn't scale; impossible for cross-cutting concerns. |
| **Trust the LLM to "not say"** | Inference leakage; prompt injection breaks it; no audit trail. |

The first three are storage problems; the fourth is a non-solution. None work for organizations with meaningful sensitivity gradients (medical, financial, legal, defense, IP-heavy).

The **right** answer (proven in IAM, databases, file systems for 40+ years): partition by **identity** and gate access by **policy**. The novelty here is applying it to **agentic retrieval**, where the "data" reaches an LLM context window — not just a database row.

### 1.2 The context-window problem

Even when sharing is not an issue, agents waste context. A coding agent fixing `login.ts` does not need the database schema, the marketing copy, or the deploy runbook. But naive retrieval (top-K by cosine similarity) returns all of them, plus 47 more atoms that mention "user" anywhere.

The cost is not just tokens — **irrelevant context produces worse outputs**:

- Hallucinated cross-references between unrelated systems
- Spurious "helpful" suggestions for fixes the user didn't ask for
- Lost focus on the actual task

Industry practice has been to throw more tokens at the problem (1M-context Claude, etc.). This spec proposes the opposite: **load less, but load it smarter**.

### 1.3 The subagent-as-attack-surface problem

When a parent agent spawns subagents (codegen, retrieval, validation), each subagent is a small LLM session with its own context window. Subagents are typically given **broader** context than they need, "just in case." This is both wasteful and dangerous:

- Wasteful: every subagent re-pays the token cost of context it never uses.
- Dangerous: a prompt-injection vector in any single atom can cause that subagent to exfiltrate everything else in its context.

The principle of least privilege (POLA) applies to agents the same way it applies to Unix processes. Subagents should see **only what their task requires** — and nothing more.

### 1.4 Why universal

A naive solution would build a medical-specific agent with hardcoded PHI tags. The next team building a financial agent re-implements the same plumbing with PCI tags. The team after that builds it again with NDA tags.

This spec instead defines **the plumbing once**, accepting that the **labels** are domain-specific. Domain knowledge enters as:

1. **Classifier plugins** — e.g. a regex matcher that tags atoms containing SSNs as `pii: true`.
2. **Policy files** — e.g. a rule saying "deny `expose-to-llm` when `pii: true` unless step-up auth completed".

This separation is the same pattern used by OPA (Open Policy Agent), Cedar (AWS), and XACML — battle-tested for 20+ years.

---

## §2 Conceptual Model — Three Axes

Every retrieval / context-loading decision in this framework can be located on three orthogonal axes:

```
                      WHO
                       ▲
                       │  (identity / role / clearance)
                       │
                       │
        ───────────────┼───────────────▶  WHERE
                       │                  (which vault / namespace / domain)
                       │
                       ▼
                  HOW MUCH
                  (resolution / depth / scope)
```

### 2.1 WHO — identity-bound retrieval

Every query carries a **Subject**: a user, a subagent task, a scheduled job, or a service. Every resource carries **owner / permission** attributes. A request is denied if Subject's clearance does not meet Resource's requirement.

This is the **security axis** — implemented via ABAC, enforced at the Policy Decision Point.

### 2.2 WHERE — vault-bound retrieval

Every query targets a **Vault** — a named composition of one or more **Namespaces**. A vault answers "out of all storage, which partitions am I allowed to look at right now?"

This is the **isolation axis** — implemented via Namespace partition keys, enforced at the storage layer (SQL `WHERE`, vector index filter).

### 2.3 HOW MUCH — resolution-bound retrieval

Every query specifies how deeply it wants to materialize the resources it retrieves. Three levels of "deep":

- **FULL** — entire content body, in context window.
- **SUMMARY** — frontmatter + first paragraph + headings.
- **SKELETON** — id + title + 1-line description.
- **MENTION** — id only, as a pointer the agent can `expand()` on demand.

This is the **economy axis** — implemented via Resolution Gradient, enforced at the composer.

### 2.4 Composability

The three axes compose multiplicatively:

```
effective_context = filter_by_WHO ∩ filter_by_WHERE ∩ shape_by_HOW_MUCH
```

A retrieval for "what does our login system look like" issued by:

- Subject `alice` (engineer, no PHI clearance)
- Vault `alice-engineering` (her personal + eng-team + company-public)
- Resolution `{ full: 3, summary: 10, skeleton: 50, budget: 8000 tokens }`

...will return at most 8000 tokens, span only those namespaces, exclude any atom alice cannot see, and grade the response by relevance.

---

## §3 Subject-Resource-Action-Context Model

The framework adopts the four-tuple model from XACML / Cedar / OPA. All access decisions take the form:

```
decide(subject, resource, action, context) → decision
```

### 3.1 Subject

A `Subject` is **anything that initiates a request**. The framework does not assume it is a human.

```ts
interface Subject {
  kind: 'user' | 'subagent' | 'service' | 'scheduled-job' | 'mcp-client'
  id: string
  attributes: AttributeBag
}
```

Examples of `attributes`:

```yaml
# Human user
{ role: ['engineer'], clearance: 'internal', auth_level: 2, mfa: true }

# Subagent task
{ task_id: 'fix-login-mfa', scope: { needs: ['auth'], excludes: ['db-schema'] } }

# Service
{ service_name: 'nightly-summarizer', oauth_scope: ['read:atoms'] }
```

### 3.2 Resource

A `Resource` is **the thing being acted upon**. In MSP this is usually an atom; it can also be an episodic entry, a vector index entry, a tool call result, or an LLM context slot.

```ts
interface Resource {
  kind: 'atom' | 'episode' | 'vector-doc' | 'tool-result' | 'context-slot'
  id: string
  namespace: Namespace
  attributes: AttributeBag
}
```

Examples of `attributes`:

```yaml
# Medical atom
{ classification: 'phi', patient_id: 'p123', hipaa_covered: true }

# Internal engineering atom
{ classification: 'internal', domain: ['auth', 'session'], team: 'security' }

# Public atom
{ classification: 'public', license: 'CC-BY' }
```

### 3.3 Action

An `Action` is **what the Subject wants to do** with the Resource.

```ts
type Action =
  | 'read'              // direct content access
  | 'recall'            // include in retrieval results
  | 'embed'             // include in vector index
  | 'expose-to-llm'     // include in LLM context
  | 'summarize'         // produce derived content
  | 'write'             // create new resource
  | 'modify'            // change existing resource
  | 'delete'
  | 'cite'              // reference by id without exposing body
```

The distinction between `recall` and `expose-to-llm` is **critical**. A document may be `recall`-able (returned as a hit) but not `expose-to-llm`-able (its body cannot enter the LLM context). This enables citation-only patterns for high-sensitivity data.

### 3.4 Context

`Context` carries **environmental** attributes that do not belong to Subject or Resource:

```ts
interface RequestContext {
  time: Date                // for time-of-day rules, age-based rules
  network?: string          // 'internal' | 'vpn' | 'public-internet'
  purpose?: string          // 'debugging' | 'production-query' | 'audit'
  scale_level?: 'L1' | 'L2' | 'L3'   // MSP scale gate
  origin: 'http' | 'mcp-stdio' | 'cli' | 'internal'
  trace_id: string
}
```

### 3.5 Decision

The PDP returns a `Decision` structure with **more than yes/no**:

```ts
interface Decision {
  effect: 'permit' | 'deny' | 'indeterminate'
  obligations: Obligation[]     // mandatory actions if permitted (redact, log, encrypt)
  advice: Advice[]              // optional recommendations (suggest narrower scope, etc.)
  reasoning: ReasonTrace[]      // explainable: which rules matched and why
  ttl_seconds?: number          // how long this decision can be cached
}
```

Obligations are first-class — a `permit` decision with `obligations: [redact-field('ssn'), log-access]` is a permission with conditions, not a blanket allow.

---

## §4 Attribute Bag — Bring-Your-Own-Schema

### 4.1 Why no fixed schema

A core that hardcodes `sensitivity: medical | financial | legal` cannot accommodate a new domain without a code change. A core that accepts an **open attribute map** can:

```ts
type AttributeBag = Record<string, JsonValue>
```

The core treats this as opaque — it carries the bag from classifier to policy engine without interpreting it. Only **policies** interpret attributes.

### 4.2 Constraining the chaos

Open schemas drift. To prevent unbounded field growth:

1. **Attribute schema files** declare expected attributes per domain:

   ```yaml
   # attributes/medical.yaml
   - name: classification
     type: enum
     values: [public, internal, confidential, phi]
   - name: patient_id
     type: string
     pattern: '^P[0-9]{6}$'
   - name: hipaa_covered
     type: boolean
   ```

2. **Policy authors declare which attributes their rules use** — the validator warns on unknown attributes:

   ```yaml
   uses_attributes: [classification, patient_id, hipaa_covered]
   ```

3. **Classifiers must publish their output schema** so the system can verify all consumers agree.

This gives you the flexibility of an open schema with the discipline of a typed one.

### 4.3 Classifier plugins

A classifier consumes a Resource and emits attributes:

```ts
interface Classifier {
  id: string
  outputs: string[]   // attribute names produced
  classify(resource: Resource): Promise<AttributeBag>
}
```

Built-in classifiers (universal):

- **`PathClassifier`** — derives `domain` from file path conventions (`gks/feat/AUTH-*.md` → `domain: auth`).
- **`ContentClassifier`** — regex-based; ships with default rules for SSN, credit card, email, API keys; **does not** assign domain labels — only sensitivity flags.
- **`GraphCommunityClassifier`** — derives `community_id` from Louvain/Leiden on the crosslink graph.
- **`EmbeddingClusterClassifier`** — derives soft `topic_id` from k-means on vectors.
- **`FrontmatterClassifier`** — passes through explicit frontmatter `attributes:` declarations.

Domain packs add classifiers, e.g.:

- **`pack-medical`** — HIPAA-aware regex + PHI tagging.
- **`pack-finance`** — PCI matchers, account-tier tagger.
- **`pack-source-code`** — secret-scanner equivalents.

### 4.4 Auto-tag vs. manual-tag precedence

When multiple classifiers tag the same attribute on the same resource, precedence is:

1. **Frontmatter** (human-authored) wins absolute.
2. **Domain pack** classifier output is next.
3. **Universal classifier** output is last.

Each tag carries `source` metadata so reviewers can see provenance:

```yaml
classification: phi
classification__source: pack-medical/v1.2.0
classification__confidence: 0.98
```

---

## §5 Namespace, Vault, Brain — Terminology and Layering

This is the most consistently misunderstood part of the system. Three distinct concepts share the rough idea of "isolation," but they operate at different layers.

### 5.1 Namespace — the partition key

A `Namespace` is the **atomic isolation unit at the storage layer**. It is the SQL `WHERE` clause that runs on every read.

```ts
interface Namespace {
  tenant_id?: string     // SaaS customer, org, project
  user_id?: string       // individual user
  session_id?: string    // ephemeral session (rare in storage; common in metadata)
  agent_id?: string      // multi-agent installations
}
```

Rules:

- **Every Resource is stamped with exactly one Namespace at creation.**
- **Every retrieval is filtered by Namespace, automatically.**
- **`crossNamespace: true` is the only bypass** and is gated to admin paths.
- **Namespaces are cheap and coarse.** They are not designed for fine-grained per-attribute rules; ABAC handles that.
- **Namespaces are immutable on a Resource.** Moving a Resource across Namespaces is a copy + retire, not a mutation.

Anti-patterns:

- Putting `domain` or `sensitivity` into Namespace — those belong in attributes.
- Using `session_id` in Namespace for permanent storage — atoms vanish when session ends.
- Per-user Namespace for shared knowledge — defeats collaboration.

### 5.2 Vault — the composition view

A `Vault` is **a named view over one or more Namespaces**. It is a UX-layer concept that answers "from the user's perspective, what storage am I working in right now?"

```ts
interface Vault {
  id: string
  name: string
  read_from: Namespace[]              // OR-union for reads
  write_to: Namespace                 // single target for writes
  default_filter?: AttributeBag       // optional default ABAC filter
  resolution_policy?: ResolutionPolicy  // see §6
}
```

Properties:

- **A user can have multiple active vaults**, unlike Obsidian where only one vault is open at a time.
- **A vault is a logical entity** — switching vaults does not move data.
- **A vault can compose across organizational boundaries** — e.g. `personal + my-team + company-public + cross-team-shared`.
- **A vault is a unit of sharing**: granting access to a vault is a clean ACL operation.
- **A vault knows its own resolution policy** — see §6.

The relationship is **N-to-N**: a Namespace can appear in many Vaults; a Vault contains many Namespaces.

### 5.3 Brain — the holistic identity

A `Brain` is **the entire active agent envelope**: identity (soul), accessible vaults, cognitive layer policy, current session.

```ts
interface Brain {
  id: string
  soul: SoulProfile                    // identity / voice / preferences (MSP existing)
  active_vaults: VaultBinding[]        // which vaults are mounted right now
  cognitive_policy: CognitivePolicy    // scale, resolution, budget, ABAC
  current_session: SessionContext
}
```

Switching brains = switching the entire context envelope. Within a brain, vaults can be added / removed / muted without losing identity.

The mental model:

| Concept | Real-world analogy | Software analogy |
|---|---|---|
| **Namespace** | A locked filing cabinet | DB row's `tenant_id` |
| **Vault** | "Documents I can access today" — could draw from many cabinets | Saved query / view |
| **Brain** | "Me, at work, on this project" — identity + accessible documents + how I work | OS user session |

### 5.4 Why three layers and not one

A common temptation is to collapse these into one concept (often called "workspace"). Resist it. The three layers serve different needs:

- **Namespace** must be cheap and immutable — it's the storage filter that runs on every query.
- **Vault** must be flexible and composable — users compose them daily, share them, switch them.
- **Brain** must persist across sessions — it carries identity / preferences / history.

Collapsing them produces either:

- An inflexible system (workspace = namespace, no sharing)
- An insecure system (workspace = vault but stored as a namespace, so policy bypass is one config flip away)
- An unobservable system (everything in one bucket, audit can't tell why something matched)

---

## §6 Resolution Gradient Retrieval

### 6.1 The problem

Naive retrieval returns top-K hits at full body. As K grows, context window fills, and most of that content is tangentially relevant — the agent "wastes attention" on it.

Human researchers do not work this way. We scan abstracts, expand the relevant ones, glance at citations, and occasionally drill into a referenced paper. **The cost of attention varies with the value of the content.**

The Resolution Gradient encodes this:

```
Tier      Content shape                          Typical tokens   MVP?
─────────────────────────────────────────────────────────────────────────
FULL      complete body + frontmatter            500 – 5000       ✅ yes
SUMMARY   frontmatter + first paragraph + h2s    50 – 300         ⏳ Phase 4
SKELETON  id + title + 1-line description        20 – 60          ⏳ Phase 4
MENTION   id only (pointer, no preview)          5 – 10           ✅ yes
```

**MVP scope (per D-2):** ship `FULL` + `MENTION` only, plus `expand()` to promote a `MENTION` to `FULL` on demand. The intermediate tiers are added in Phase 4 once telemetry confirms `expand()` is exercised frequently enough to justify the renderer / budget-allocator complexity. The composer must, however, **structure data as if all four tiers exist** so adding `SUMMARY` / `SKELETON` later is a renderer addition, not a rearchitecture.

### 6.2 Tiering signals

Which tier a Resource receives is computed from:

1. **Direct match score** (vector similarity / BM25) — primary signal.
2. **Graph hop distance** from seed Resources — secondary signal.
3. **Vault link policy** — cross-vault links may cap maximum tier (e.g. "from `company-public`, never load `FULL`").
4. **Resource attributes** — sensitivity may force a tier (e.g. `pii: true` → max `MENTION` unless step-up).
5. **Budget pressure** — when the total budget is exceeded, the lowest-relevance Resources drop tiers first.

A reference implementation of the tier assignment function:

```
score(resource) = w1 * similarity + w2 * 1/(1 + hops)
                  capped by min(vault_cap, attribute_cap)

tier(resource) =
  | FULL      if score in top-N1 and ≥ threshold_full
  | SUMMARY   if score in top-N2 and ≥ threshold_summary
  | SKELETON  if score in top-N3 and ≥ threshold_skeleton
  | MENTION   if score in top-N4
  | (omitted) otherwise
```

`N1..N4` and thresholds are configured per vault and overridable per query.

### 6.3 Budget allocation

Per-tier budget allocation prevents one tier from starving another:

```yaml
budget:
  total_tokens: 8000
  per_tier:
    FULL:     50%   # may overflow into SUMMARY share if N1 < target
    SUMMARY:  30%
    SKELETON: 15%
    MENTION:   5%
  on_overflow: compress_full_first   # don't starve lower tiers
```

`on_overflow` strategies:

- `compress_full_first` — abridge FULL bodies before dropping any SKELETON.
- `drop_lowest_first` — keep FULL intact, drop MENTION first.
- `proportional` — scale all tiers' share.

### 6.4 Skeleton content

Skeletons must carry enough information for the agent to decide whether to `expand()`. The mandatory fields:

```
[id]  [title]
  └─ [1-line description from frontmatter `description:` or first heading]
```

Optional enrichments configurable per vault:

- Tags / domain
- Last-updated date
- Direct crosslink count (signal of centrality)

### 6.5 Expand-on-demand

The agent (or user, via UI) can request promotion of any Resource to a higher tier:

```ts
const detail = await layer.expand('CONCEPT--ANTI-HALLUCINATION', {
  to: 'FULL',
  reason: 'investigating contradiction'
})
```

Properties:

- **Cost is predictable** — the caller knows token count before the call returns.
- **Calls are logged** — audit trail records which resources got promoted and why.
- **Policy still applies** — `expand()` re-runs ABAC; a Resource may be skeleton-only because ABAC denied FULL access, not because it was distant.
- **Bounded** — vaults may set max-expansions-per-query to prevent runaway loops.

### 6.6 Comparison to existing techniques

| Technique | What it does | What this adds |
|---|---|---|
| **Top-K vector retrieval** | Returns K results, all FULL | Single resolution; no graph awareness |
| **Multi-hop retrieval (HippoRAG)** | Expands through graph hops | Same resolution for all; no compression |
| **Self-RAG** | Agent decides to retrieve more | Compatible — expand() is the same idea |
| **Late-chunking / RAPTOR** | Hierarchical summary trees | Static hierarchy; not query-adaptive |
| **LSP hover / go-to-definition** | Signature → body on click | UI-level; no batch retrieval integration |

Resolution Gradient = **graph topology + tiered compression + agent autonomy + per-tier budget**. The combination is novel; the parts are individually known.

---

## §7 ABAC Policy Engine

### 7.1 PDP / PEP separation

The framework uses the classic XACML / OPA architecture:

```
                        ┌────────────────────────┐
   Request (4-tuple) ──▶│   PEP (Enforcement)    │
                        │   per transport         │
                        └───────────┬────────────┘
                                    │ ask
                                    ▼
                        ┌────────────────────────┐
                        │   PDP (Decision)       │
                        │   domain-agnostic      │
                        └───────────┬────────────┘
                                    │ consult
                                    ▼
                        ┌────────────────────────┐
                        │   Policy Store          │
                        │   YAML / Cedar / OPA    │
                        └────────────────────────┘
```

- **PEP** — Policy Enforcement Point. One per entry surface (HTTP middleware, MCP tool wrapper, recall interceptor, embed interceptor, composer). Stops execution if PDP denies.
- **PDP** — Policy Decision Point. Pure function: `(subject, resource, action, context) → Decision`. Has no I/O, no side effects.
- **Policy Store** — declarative policy files, hot-reloadable. Not code.

### 7.2 Why policy as data, not code

| Code-based rules | Data-based rules |
|---|---|
| Each change requires deploy | Hot-reload |
| Rules scattered across modules | One source of truth |
| Hard to audit / explain | Decision trace is structural |
| Hard to test in isolation | Fixture-driven tests |
| Domain teams need engineers | Domain teams can author rules |

### 7.3 Policy file format (initial)

**Format (per D-1):** YAML with a small built-in operator set. Estimated 200 LOC for parser + evaluator. Hot-reload is supported via file watcher + monotonic policy-version counter. Migration to Cedar / OPA is a mechanical translation once policy count or expressiveness needs exceed the YAML form (target threshold: ~30 rules or any need for recursive evaluation).

# policies/subagent-context-scoping.yaml

- id: deny-out-of-scope-atoms
  description: Subagent tasks see only atoms whose domain is in scope.needs.
  uses_attributes: [domain]
  match:
    subject.kind: subagent
    action: expose-to-llm
  condition: |
    resource.attributes.domain ∩ subject.attributes.scope.needs ≠ ∅
    AND resource.attributes.domain ∩ subject.attributes.scope.excludes = ∅
  effect: permit
  fallthrough: deny
  obligations: [log-scope-decision]

- id: medical-needs-step-up
  description: PHI atoms require step-up auth in the last 5 minutes.
  uses_attributes: [classification]
  match:
    resource.attributes.classification: phi
    action: [read, expose-to-llm]
  condition: |
    subject.attributes.auth_level ≥ 3
    AND context.time - subject.attributes.last_step_up_at ≤ 5min
  effect: permit
  obligations: [log-phi-access, redact-fields-except-citation]
  on_deny:
    advice: ['request-step-up-auth']

```

Operator language is intentionally minimal at v1 — equality, membership, set operations, basic arithmetic. Cedar / Rego is the upgrade path when expressiveness is needed.

### 7.4 Decision caching

Per-`(subject, resource, action)` hash, decisions cache for `decision.ttl_seconds`. Policy changes invalidate the cache via a monotonic policy-version counter included in the cache key.

### 7.5 Explainability

Every decision must produce a `reasoning: ReasonTrace[]` array — which rules matched, which conditions held, which obligations attached. This is non-negotiable: opaque deny decisions are useless to operators and break compliance audits.

### 7.6 PEP integration points in MSP

| Entry point | PEP location | Actions checked |
|---|---|---|
| `POST /index` (Express) | HTTP middleware | `write` |
| `POST /recall` (Express) | HTTP middleware | `recall`, `expose-to-llm` |
| MCP tool calls | per-tool wrapper in `src/mcp/tools/` | tool-specific |
| `layer.recall()` (facade) | interceptor in `src/cognitive/` | `recall`, `expose-to-llm` |
| `layer.runTask()` (facade) | interceptor in `src/codegen/master/composer.ts` | `expose-to-llm` (filter atoms entering subagent context) |
| `layer.remember()` (facade) | interceptor in `src/cognitive/` | `write` |
| GKS vector index | embedder interceptor | `embed` |

---

## §8 Step-up Authentication

### 8.1 When step-up is required

Step-up auth requires the Subject to re-prove identity within a short window before performing a specific action. It is **not** a replacement for primary login; it is a defense-in-depth measure for high-sensitivity actions even when the outer session is valid.

Step-up should fire when:

1. **Resource sensitivity exceeds a threshold** (e.g. `classification: phi` or `restricted`).
2. **Action is high-impact** (e.g. `delete`, `write` to a privileged namespace, cross-namespace promote).
3. **Risk context is elevated** (new device, off-hours, novel IP, sudden volume).
4. **Last step-up is older than the action's TTL** (e.g. ≤ 5 min for PHI, ≤ 30 min for general internal).

### 8.2 Step-up mechanisms

| Mechanism | UX cost | Security | Phishable | Bound to prompt? |
|---|---|---|---|---|
| **PIN re-entry** | low | low | yes | no |
| **TOTP (Authenticator)** | medium | medium | yes | no |
| **Push notification** | low | medium | medium | no |
| **Passkey / WebAuthn signature** | low | high | no | optional |
| **Signed prompt token** (HMAC/ES256 over prompt+nonce+ts) | minimal once provisioned | high | no | **yes** |
| **Hardware key (FIDO2)** | medium | very high | no | optional |

Recommended for MSP v1:
- **Local / single-user / homelab** → PIN re-entry, optionally TOTP.
- **Team / production / sensitive data** → Passkey or signed prompt token. Both bind to the prompt, preventing replay and prompt-injection-style hijack.

### 8.3 Step-up flow

```

1. Subject requests action A on Resource R.
2. PDP evaluates → returns deny with advice: ['request-step-up-auth', ttl: 300s].
3. PEP intercepts → triggers step-up UI / MCP step-up tool.
4. User completes mechanism (e.g. signs WebAuthn challenge).
5. Backend records subject.attributes.last_step_up_at = now.
6. Subject retries action A → PDP now permits → action proceeds.
7. Audit log records both deny+reason and subsequent permit+step-up.

```

### 8.4 Step-up over MCP stdio

MCP stdio is local-trust by design. There is no browser to prompt for biometrics. Options:

- **Out-of-band confirmation** — MCP server emits a notification to a paired web/mobile session; user confirms there; MCP call blocks until confirmation arrives or times out.
- **Pre-signed token bundles** — a CLI command issues N step-up tokens; MCP calls consume them; user re-signs when exhausted.
- **Per-tool risk class** — MCP tools annotated `risk: high` always route through an out-of-band confirmation; `risk: low` tools never require step-up.

This is one of the open questions — see §14.

---

## §9 Subagent Context Scoping

### 9.1 The model

When a parent agent spawns a subagent (codegen task, retrieval task, validation task), the parent declares the subagent's **scope of concern**. The scope becomes part of the subagent's `Subject.attributes` and is consulted by every recall and compose operation in the subagent's lifetime.

```ts
interface SubagentScope {
  needs: string[]           // domains the subagent requires
  nice_to_have: string[]    // load if budget permits
  excludes: string[]        // explicitly forbidden; never load
  budget_tokens: number
  allow_expand: boolean     // can subagent expand() resources?
  expand_limit?: number     // max expansions per task
}
```

Example task descriptor:

```yaml
# .brain/tasks/FIX-LOGIN-MFA/T1.task.yaml
task:
  id: fix-login-mfa
  goal: "Add passkey support to login flow"
scope:
  needs: [auth, session, middleware]
  nice_to_have: [identity, telemetry]
  excludes: [schema-design, db-migration, ui-styling, billing, patient-records]
budget:
  tokens: 8000
  allow_expand: true
  expand_limit: 5
```

### 9.2 The effective-context formula

For any candidate Resource `R` being considered for inclusion in the subagent's context:

```
include(R, subagent) =
    namespace_filter(R, subagent.vault)           # storage isolation
  ∩ user_abac_filter(R, parent_user)              # user-level ABAC
  ∩ task_scope_filter(R, subagent.scope)          # task-level filter
  ∩ resolution_tier(R, score, budget) > omitted   # made it past budget
```

All four conditions must hold. The order matters for cost: namespace filter is cheapest (indexed `WHERE`), so it runs first; resolution tiering is most expensive (involves graph traversal + scoring), so it runs last.

### 9.3 The escalation pattern

If a subagent's filter is too restrictive, it does not silently produce a bad answer — it **escalates**:

```ts
// Inside subagent context
const detail = await layer.expand('PROBABLE-RELEVANT-ATOM-ID', {
  reason: 'need to verify how login handles session expiry',
  to: 'FULL'
})

if (detail.denied_reason === 'out_of_scope') {
  // Subagent surfaces to parent:
  await layer.escalate({
    request_scope_extension: ['session-management'],
    reason: 'login fix requires understanding session expiry path'
  })
  // Parent decides: approve, deny, or expand task scope.
}
```

The escalation pattern is **load-bearing**. Without it, scope filtering produces silent quality drops instead of loud errors. With it, restrictions are observable and adjustable.

### 9.4 Why subagents do not set their own scope

A subagent given the ability to declare its own scope will always declare the maximum scope "to be safe." This defeats the purpose. Scope is set by the parent (or by the task author), and the subagent operates within it; the only autonomy the subagent has is to **request** scope extensions via escalation.

This mirrors Unix process privilege: `setuid` allows declared elevation, not arbitrary self-promotion.

### 9.5 Defense-in-depth with user ABAC

The chain `user_abac ∩ task_scope` is the key defense-in-depth property. Concrete example:

> Dr. Alice is logged in (authorized to view PHI). She launches a subagent to fix the login bug. The subagent's `excludes` list includes `patient-records`.
>
> Even though Alice's session would permit access to patient records, the task scope forbids them. The subagent **cannot** see them — eliminating the risk that a prompt injection in any atom causes the subagent to exfiltrate patient records.

This is **the most valuable property of the framework**. User-level access control alone cannot achieve it.

---

## §10 Composition Pipeline — Five Layers

A complete retrieval / context-loading request flows through five layers, in order:

```
Query (subject, vault, scope, budget)
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Layer 1: Namespace Filter                                  │
│ Reduces candidate set from all storage to vault membership │
│ Cost: O(log n) — indexed                                   │
│ Drop rate: 90-99% (typical)                                │
└────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Layer 2: ABAC Policy Filter (PDP per Resource)             │
│ Drops Resources whose attributes Subject cannot access     │
│ Cost: O(n_after_layer1) — policy eval per Resource         │
│ Drop rate: 0-50% depending on policy                       │
└────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Layer 3: Graph + Vector Scoring                            │
│ Scores remaining Resources by similarity + graph topology  │
│ Cost: O(n_after_layer2 * k_neighbors)                      │
│ Drop rate: 0% (just sorts and assigns hop distance)        │
└────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Layer 4: Resolution Tier Assignment                        │
│ Assigns FULL/SUMMARY/SKELETON/MENTION based on score+caps  │
│ Cost: O(n_after_layer2)                                    │
│ Compression rate: 80-95% of tokens cut                     │
└────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Layer 5: Budget Enforcement                                │
│ Trims to fit token budget; demotes/compresses as needed    │
│ Cost: O(n_after_layer4) sort + accumulate                  │
│ Result: a budget-fitting context bundle with citations     │
└────────────────────────────────────────────────────────────┘
  │
  ▼
Context bundle → LLM call (or returned to caller)
```

### 10.1 Order rationale

Each layer is positioned where it is most efficient:

- **Namespace first** — it is the cheapest, indexed, and drops the most candidates.
- **ABAC second** — runs only on the survivors; benefits from a small input.
- **Scoring third** — needs the filtered set to compute relative scores.
- **Resolution fourth** — depends on scores being final.
- **Budget last** — operates on the final set.

Re-ordering breaks correctness or performance. ABAC before namespace, for instance, would invoke the PDP on resources the user could never see anyway — wasted CPU. Scoring before ABAC would mean the LLM sees relevance from atoms it isn't allowed to access (information leakage through cache effects).

### 10.2 Layer ownership

| Layer | Owned by | Files |
|---|---|---|
| Layer 1 | GKS | `packages/gks/src/memory/vector/`, `packages/gks/src/memory/gks.ts` |
| Layer 2 | New (universal-policy module) | `packages/msp/src/policy/` (new) |
| Layer 3 | GKS + MSP | `packages/gks/src/memory/graph/`, `packages/msp/src/orchestrator/retrieval/` |
| Layer 4 | MSP (new) | `packages/msp/src/orchestrator/resolution/` (new) |
| Layer 5 | MSP | `packages/msp/src/orchestrator/compressor/` (existing — extend) |

---

## §11 Phased Implementation

Each phase ships behind a feature flag, is independently testable, and produces a usable improvement on its own.

### Phase 0 — Plumbing (Week 1)

**Deliverables:**

- Add `Subject`, `Resource`, `Action`, `Context` types to MSP.
- Add `AttributeBag` field to atom frontmatter (optional, ignored by current validator).
- Thread Subject and Action through `recall()`, `retain()`, `runTask()`, and MCP tool handlers. No enforcement — just propagation and logging.

**Acceptance:**

- All entry points log full 4-tuple on every call.
- Type-check passes; existing tests pass.
- No behavior change for end users.

**Risk:** very low.

### Phase 1 — Policy Decision Point, log-only mode (Week 2)

**Deliverables:**

- Implement `evaluatePolicy(subject, resource, action, context): Decision` (~300 LOC).
- YAML policy loader with hot reload (per D-1).
- Default posture: **`default-permit`** (per D-7) — every action is permitted unless an explicit rule denies it.
- One starter policy: `log-everything-permit-everything` (literally permits and logs every call).
- PEP at `layer.runTask()` only; in **shadow mode** — PDP runs, decisions are logged with full reasoning, but enforcement is off (every action runs regardless of decision).

**Acceptance:**

- Decision logs include reasoning trace.
- Operator can author a new YAML policy and see it pick up without restart.
- `runTask` still produces identical outputs (shadow mode confirmed).
- The shadow log surfaces "what would have been denied if we flipped to enforce" — this is the key artifact for Phase 2 confidence.

**Risk:** low. Worst case the PDP misbehaves — fall back to log-only is automatic.

### Phase 2 — Subagent scope filtering, enforced (Week 3)

**Deliverables:**

- Task descriptor schema (`scope.needs`, `scope.excludes`).
- PEP at composer (`src/codegen/master/composer.ts`) enforces task scope filter.
- Domain tagging for top-20 atoms in this repo (manual; serves as test bed).
- Quality A/B: same task with/without scope filtering; compare outputs.

**Acceptance:**

- Scope-filtered runs produce comparable or better outputs at 30-50% fewer tokens.
- Escalation works: subagent can request scope extensions; parent logs decisions.

**Risk:** medium — quality regression possible if scope rules are wrong.

### Phase 3 — Vault and resolution gradient (Week 4-5)

**Deliverables (per D-2 — 2-tier MVP):**

- Vault config files (`~/.msp/vaults/*.yaml`).
- Layer 4 (resolution tier assignment) + Layer 5 (budget enforcement) in composer.
- `expand()` MCP tool + facade method (promote a `MENTION` to `FULL`).
- **Tier set:** `FULL` and `MENTION` only. `SUMMARY` and `SKELETON` deferred to Phase 4 — but the tier-assignment data model must already encode all four so adding renderers later is additive.
- First per-endpoint flip from `default-permit` → `default-deny` for the `expose-to-llm` action on Resources marked `restricted` or higher (per D-7).

**Acceptance:**

- Token usage on standard query set drops 60%+ vs. flat top-K.
- Agents call `expand()` and the call chains work end-to-end.
- `default-deny` for `restricted` resources confirmed in audit log.

**Risk:** medium — risk of breaking existing recall consumers.

### Phase 3.5 — SUMMARY / SKELETON renderers (optional, gated on telemetry)

**Trigger to ship:** Phase 3 telemetry shows `expand()` is called on ≥ 20% of MENTION-tier resources. Below that threshold, the cost of building renderers exceeds the value — agents are happy with FULL + MENTION.

**Deliverables:**

- `SummaryRenderer` — extracts frontmatter + first paragraph + heading list.
- `SkeletonRenderer` — extracts id + title + 1-line description from frontmatter.
- Per-tier budget allocator.
- Vault config: per-tier caps and `on_overflow` strategy.

**Acceptance:** token savings on same standard query set increase by another 20-30% beyond Phase 3 baseline.

**Risk:** low — additive, behind a flag.

### Phase 4 — User-level ABAC, enforced (Week 6) — ✅ COMPLETED

**Deliverables:**

- `Subject` populated from authenticated user (Express middleware + MCP per-call identity).
- User attribute store (`roles`, `clearance`, `mfa_status`, `tenant_ids`).
- Default policy pack: `pack-multi-tenant`, `pack-pii-block-from-llm`.
- PEP at all read entry points (`recall`, `expand`, `compose`).

### Phase 5 — Step-up authentication (Week 7) — ✅ COMPLETED

**Deliverables:**

- Step-up provider interface (`packages/msp/src/policy/step-up/provider.ts`).
- PIN-based provider (built-in) using `scrypt` for platform-stable hashing.
- Secure in-memory challenge store with TTL and replay defense.
- `msp-auth` CLI for PIN management.

### Phase 6 — Classifier plugins + auto-tagging (Week 8+) — ✅ COMPLETED

**Deliverables:**

- Classifier plugin interface and engine with provenance tracking.
- Built-in classifiers: `PathClassifier`, `ContentClassifier` (PII/Secrets), `CodingClassifier`, `TaskClassifier`, `SecurityClassifier`.
- `msp-tag` CLI for batch tagging.
- Full vault scan complete: 392 atoms tagged with domain and security metadata.

---

## §12 Threat Model & Landmines

### 12.1 Embedding leakage

**Threat:** A vector index contains embeddings of all atoms regardless of sensitivity. A user without PHI clearance issues a query; the index returns top-K hits including PHI atoms. ABAC drops them post-retrieval, but timing information leaks existence ("there's a PHI atom near your query").

**Mitigations:**

1. **Filter at the vector index layer**, not after — pgvector `WHERE` on `attributes->>'classification'`.
2. **Separate indexes per sensitivity tier** for the most sensitive data.
3. **Constant-time deny paths** — match the timing of a "no results" with the timing of a "denied results."

### 12.2 Episodic memory contamination

**Threat:** Episodic memory summarizes prior session content. If a privileged user's session generated an episodic entry about PHI, a less-privileged user's later session may retrieve that episodic entry (it has no inherent classification).

**Mitigations:**

1. **Inherit attributes from source atoms** when generating episodic entries.
2. **Scope episodic per Subject** (current MSP scope is per-workspace; needs an additional `user_id` axis).
3. **Re-evaluate episodic against current Subject's policy** on retrieval.

### 12.3 LLM inference leak

**Threat:** Even with perfect retrieval ABAC, the LLM may infer protected information from non-protected context (e.g. "given the patient's symptoms in the abstract, this is probably a rare disorder").

**Mitigations:**

1. **Never expose raw sensitive data to LLM context** — prefer citation patterns where the LLM receives de-identified abstracts + reference ids, and resolution happens out-of-band at the UI / data layer.
2. **Treat LLM responses as untrusted output** — scan for unintended PII / sensitive patterns before returning to users.
3. **Provider-side data minimization** — for hosted models (Anthropic, OpenAI), ensure no PHI traverses third-party APIs; use local models for high-sensitivity tasks.

### 12.4 Skeleton title leak

**Threat:** ABAC denies FULL access to an atom but allows SKELETON. The skeleton includes title and id, which may themselves leak sensitive information ("PATIENT-JOHN-DOE-DIAGNOSIS").

**Mitigations:**

1. **ABAC runs before resolution tier assignment.** Deny decisions block all tiers, not just FULL.
2. **Per-tier policy support** — a policy can permit MENTION while denying SKELETON.
3. **Redacted skeleton rendering** — replace sensitive parts of titles with placeholders (`Patient ████ Diagnosis`).
4. **Avoid sensitive identifiers in atom titles** as a content-authoring convention.

### 12.5 Symbol-community drift

**Threat:** Auto-tagging by graph community is unstable. Adding a new crosslink can flip an atom from `domain: auth` to `domain: identity`, silently changing which subagents can see it.

**Mitigations:**

1. **Pin community version in vault config**; recompute on explicit `re-tag` command, not on every commit.
2. **Log all auto-tag changes**; reviewer must approve drift in PR.
3. **Manual overrides win** — once a human tags an atom, auto-tag suggests but does not overwrite.

### 12.6 Over-restrictive scope = silent quality loss

**Threat:** Task scope `excludes: [db-schema]` blocks atoms the subagent actually needed. Subagent produces a bad answer; no error is raised.

**Mitigations:**

1. **Escalation is mandatory.** Subagent must surface "I might be missing context X" rather than silently failing.
2. **Quality A/B** as gating in Phase 2 — ship scope filtering only if it doesn't regress task success rate.
3. **`nice_to_have` tier** lets some "maybe relevant" context in without committing to `needs`.

### 12.7 Policy explosion

**Threat:** Each team / domain authors their own policies; eventually the policy set is unauditable, conflicting, and slow to evaluate.

**Mitigations:**

1. **Decision explanation is mandatory** — auditor can always ask "why?"
2. **Dry-run mode** — operators test new policies against historical decisions before enforcing.
3. **Decision caching** — repeated identical queries don't re-evaluate.
4. **Linting** — detect contradictory rules at policy-load time.

### 12.8 Step-up replay

**Threat:** Captured step-up token is replayed on a different prompt.

**Mitigations:**

1. **Bind step-up to prompt hash + nonce + timestamp** — token is one-shot.
2. **Short TTL** — 5 minutes for high-sensitivity.
3. **Server-side replay log** — reject seen nonces.

### 12.9 Cross-namespace promote bypass

**Threat:** Promoting an atom from `personal` to `team` shared namespace removes per-user isolation. A malicious user promotes without authorization.

**Mitigations:**

1. **Promote is its own action** with a dedicated policy hook.
2. **Promote requires both source-namespace `read` and target-namespace `write`.**
3. **Promote is audited heavily** — diff included in audit log.

---

## §13 Glossary

| Term | Definition |
|---|---|
| **ABAC** | Attribute-Based Access Control. Access decisions based on attributes of Subject, Resource, Action, Context — not just role identity. |
| **Action** | What the Subject wants to do with the Resource (read, recall, expose-to-llm, write, etc.). |
| **Atom** | A structured markdown unit in `gks/<type>/*.md`. Single concept / decision / feature; validated; crosslinked. |
| **Attribute Bag** | Open-schema key-value map carrying domain-specific metadata. Bring-your-own. |
| **Brain** | The whole active agent envelope: identity (soul) + active vaults + cognitive policy + session. |
| **Classifier** | A plugin that consumes a Resource and emits attributes. Path-based, regex-based, ML-based, graph-based, or domain-pack. |
| **Context** | Environmental request attributes (time, network, purpose, scale level, trace id). |
| **Decision** | The PDP's response: effect (permit/deny/indeterminate) + obligations + advice + reasoning trace. |
| **Domain Pack** | A bundle of classifiers + policies for a specific industry (medical, finance, legal). |
| **Expand** | Promote a Resource from a lower resolution tier to a higher one on demand. |
| **MENTION** | Resolution tier: id only, as a pointer for `expand()`. ~5–10 tokens. |
| **Namespace** | Storage-layer partition key. Composite of tenant/user/session/agent ids. Immutable per Resource. |
| **Obligation** | A mandatory action attached to a permit decision (redact, log, encrypt). |
| **PDP** | Policy Decision Point. Pure function: `(subject, resource, action, context) → Decision`. |
| **PEP** | Policy Enforcement Point. Per-transport interceptor that consults PDP and enforces the result. |
| **Resolution** | The amount of Resource content rendered into context: FULL / SUMMARY / SKELETON / MENTION. |
| **Resource** | The thing being acted upon (atom, episode, vector doc, tool result, context slot). |
| **Skeleton** | Resolution tier: id + title + 1-line description. ~20–60 tokens. |
| **Soul** | Identity, voice, preferences. The MSP "passport" concept. Persists across sessions and vaults. |
| **Step-up** | Re-authentication required before a sensitive action, even with a valid outer session. |
| **Subject** | The initiator of a request: user, subagent, service, scheduled job. |
| **SUMMARY** | Resolution tier: frontmatter + first paragraph + headings. ~50–300 tokens. |
| **Vault** | A named view over one or more Namespaces. UX-layer concept for sharing and composition. |

---

## §14 Open Questions

Resolved questions have moved to §0. The following remain open. Each is annotated with the phase by which a decision is required.

1. **Hop metric.** Pure graph hops, pure cosine, or weighted sum? Each has correctness edge cases. **Required by Phase 3.** Working assumption: weighted hybrid `0.7·sim + 0.3·(1/(1+hops))` — revise after Phase 2 telemetry.

2. **MCP step-up channel.** Out-of-band web/mobile confirmation, pre-signed token bundles, or per-tool risk class? Each has UX trade-offs. **Required by Phase 5.** Working assumption: per-tool risk class with out-of-band only for `risk: high` tools.

3. **Auth provider.** Build minimal (PIN + passkey ourselves) vs. integrate an IdP (Auth0, Keycloak, Authentik) vs. delegate to reverse proxy (oauth2-proxy, Caddy)? **Required by Phase 4.** Working assumption: minimal in-house (PIN + Passkey) + accept reverse-proxy headers as an alternative auth source.

4. **Vault membership versioning.** When Alice leaves the team, atoms she wrote remain in the team vault. Do we recompute her vault view at every query (slow but correct) or snapshot membership at session start (fast but stale)? **Required by Phase 3.** Working assumption: session-snapshot with explicit `refresh` action.

5. **Audit log destination.** Inline in MSP audit (current pattern) vs. dedicated security log with stricter retention? Compliance teams will want the latter; operations teams may want correlation with existing logs. **Required by Phase 4.** Working assumption: inline MSP audit with `audit_class: security` field, split file later if a regulator requires.

6. **Embedding-leak threshold.** What is acceptable timing leakage? Constant-time everywhere is expensive; "good enough" tolerances need defining per deployment class. **Required by Phase 4 for sensitive-vault deployments.** Working assumption: constant-time deny only when vault config sets `sensitive_mode: true`.

---

## §15 References & Crosslinks

### Existing MSP / GKS atoms (load-bearing for this spec)

- `gks/frame/FRAMEWORK--MSP-ARCHITECTURE-V2.md` — top-level MSP architecture.
- `gks/concept/CONCEPT--AGENT-AGNOSTIC.md` — agent-agnostic boundary contract.
- `gks/concept/CONCEPT--MSP-ROADMAP.md` — milestone plan.
- `docs/gks/adr/004-namespace-as-first-class.md` — Namespace primitive.
- `docs/gks/adr/008-gks-storage-engine-scope.md` — GKS as storage engine.
- `docs/gks/adr/009-msp-as-orchestrator.md` — MSP as orchestrator.
- `FRAMEWORK_MASTER_SPEC.md` — master spec; this document complements it, does not replace it.

### Atoms to be produced from this spec (P1–P3 of doc-to-code)

- `FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK` — architecture summary.
- `CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT` — 4-tuple model (§3).
- `CONCEPT--ATTRIBUTE-BAG-MODEL` — open attribute schema (§4).
- `CONCEPT--NAMESPACE-VAULT-BRAIN` — three-layer terminology (§5).
- `CONCEPT--RESOLUTION-GRADIENT` — graded retrieval (§6).
- `CONCEPT--ABAC-POLICY-ENGINE` — PDP/PEP separation (§7).
- `CONCEPT--STEP-UP-AUTH` — defense-in-depth auth (§8).
- `CONCEPT--SUBAGENT-CONTEXT-SCOPING` — task-level POLA (§9).
- `ADR--POLICY-AS-DATA-NOT-CODE` — YAML/Cedar/OPA choice.
- `ADR--BRING-YOUR-OWN-ATTRIBUTES` — open schema choice.
- `ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT` — PEP per surface.
- `ADR--RESOLUTION-TIER-COUNT` — 2 vs 4 (see §14).
- `ADR--HOP-METRIC-FORMULA` — scoring choice (see §14).
- `ADR--VAULT-NAMESPACE-LAYERING` — three-layer rationale.
- `ADR--DEFAULT-POLICY-POSTURE` — default-deny vs default-permit (see §14).
- `FEAT--POLICY-DECISION-POINT` — PDP implementation.
- `FEAT--VAULT-COMPOSITION` — multi-namespace view.
- `FEAT--RESOLUTION-EXPAND-ON-DEMAND` — `expand()` tool + facade.
- `FEAT--SUBAGENT-SCOPE-FILTERING` — task-scope-aware composer.
- `FEAT--STEP-UP-AUTH-PIN` — minimal step-up provider.
- `BLUEPRINT--PHASE-0-PLUMBING` — propagation only.
- `BLUEPRINT--PHASE-1-PDP-SHADOW` — log-only mode.
- `BLUEPRINT--PHASE-2-SUBAGENT-SCOPING` — first enforcement point.
- `BLUEPRINT--PHASE-3-VAULT-AND-RESOLUTION` — vaults + tiered retrieval.

### External references

- **XACML** — eXtensible Access Control Markup Language. The progenitor of the Subject/Resource/Action/Context model.
- **OPA / Rego** — Open Policy Agent. Production-proven PDP architecture; potential v2 backend for our policy engine.
- **Cedar** — AWS's policy language; more ergonomic than Rego for ABAC.
- **WebAuthn / FIDO2** — Passkey standards; recommended step-up mechanism.
- **NIST SP 800-162** — Guide to ABAC; informs §3 and §7.
- **HippoRAG (Gutiérrez et al., 2024)** — multi-hop retrieval over knowledge graphs.
- **Self-RAG (Asai et al., 2023)** — agent-directed retrieval; informs §6.5.
- **GraphRAG (Microsoft, 2024)** — community-detection-based retrieval; informs auto-tagging.

---

## Change Log

| Version | Date | Change |
|---|---|---|
| `0.1.0` | 2026-05-13 | Initial draft consolidating chat-derived design. |
| `0.1.1` | 2026-05-13 | Resolved D-1 (YAML policy lang), D-2 (2-tier MVP), D-7 (default-permit + shadow log), D-8 (attributes in metadata, Namespace untouched). Added §0 Resolved Decisions and Phase 3.5 for SUMMARY/SKELETON renderers. Updated §6, §7, §11 Phase 1 / Phase 3 inline. Trimmed §14 open questions accordingly. |

---

*End of UNIVERSAL CONTEXT FRAMEWORK specification.*
derived design. |
| `0.1.1` | 2026-05-13 | Resolved D-1 (YAML policy lang), D-2 (2-tier MVP), D-7 (default-permit + shadow log), D-8 (attributes in metadata, Namespace untouched). Added §0 Resolved Decisions and Phase 3.5 for SUMMARY/SKELETON renderers. Updated §6, §7, §11 Phase 1 / Phase 3 inline. Trimmed §14 open questions accordingly. |

---

*End of UNIVERSAL CONTEXT FRAMEWORK specification.*
