---
id: FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
phase: 0
type: framework
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Universal context framework — identity-aware, policy-controlled,
  graded-resolution retrieval
tags: &a1
  - msp
  - framework
  - ucf
  - context
  - abac
  - vault
  - resolution
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    - CONCEPT--ATTRIBUTE-BAG-MODEL
    - CONCEPT--NAMESPACE-VAULT-BRAIN
    - CONCEPT--COGNITIVE-LAYER-FACADE
    - CONCEPT--KNOWLEDGE-LAYERS-V2
created_at: 2026-05-13T08:59:37.161+07:00
aliases: &a3
  - FRAMEWORK
  - implementation_flow
  - Governance / architectural framework
cluster: implementation_flow
role: Governance / architectural framework
attributes:
  id: FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
  phase: 0
  type: framework
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Universal context framework — identity-aware, policy-controlled,
    graded-resolution retrieval
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T08:59:37.161+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Governance / architectural framework
  attributes:
    id: FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    phase: 0
    type: framework
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Universal context framework — identity-aware, policy-controlled,
      graded-resolution retrieval
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T08:59:37.161+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Governance / architectural framework
    attributes:
      domain: framework
    domain: framework
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: framework
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# FRAME — Universal Context Framework

> Complements `[[FRAME--MSP-ARCHITECTURE-V2]]`. v2 defined the **three-layer** stack (cognitive layer / Memory OS / knowledge base). This FRAME defines how, within that stack, every retrieval and tool-call decision is shaped by **three orthogonal axes** — WHO, WHERE, and HOW MUCH — and how those axes compose into a single pipeline.
>
> Full narrative: [`docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md`](../../docs/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md).

## Pattern

Every context-shaping operation in MSP (recall, retain, expose-to-llm, embed, runTask) is parameterised by a **four-tuple**:

```
decide(subject, resource, action, context) → decision
```

where Subject, Resource, Action, Context are independent dimensions whose values are open `AttributeBag`s, and the decision carries permit/deny + obligations + reasoning. The decision is consumed by a **five-layer pipeline**:

```
┌─ Layer 1 ─ Namespace filter ─────────── storage partition (cheap, indexed)
├─ Layer 2 ─ ABAC policy filter ────────── PDP per Resource
├─ Layer 3 ─ Graph + vector scoring ────── relevance topology
├─ Layer 4 ─ Resolution tier assignment ── FULL / SUMMARY / SKELETON / MENTION
└─ Layer 5 ─ Budget enforcement ─────────── token ceiling
```

The five layers are ordered by cost (cheapest first) and by dependency (each layer needs the previous one's output). Re-ordering breaks correctness, performance, or both.

### The three axes

- **WHO** — the Subject and Resource's identity / clearance attributes (security axis). Enforced via the **ABAC** policy engine.
- **WHERE** — the active Vault and the Resource's Namespace (isolation axis). Enforced via the **storage partition** in GKS.
- **HOW MUCH** — the Resolution tier and budget (economy axis). Enforced via the **composer** in MSP.

The axes are independent and composable: any decision can be located on all three at once, and the final context bundle is their intersection.

## When to apply

This FRAME is the entry point for any feature that:

1. Adds new entry points to MSP (HTTP routes, MCP tools, facade methods) — they must accept Subject + Action and consult the PDP.
2. Introduces new Resource types into the knowledge base — they must carry an `AttributeBag` and stamp a `Namespace`.
3. Composes context to send to an LLM — they must run the five-layer pipeline before composing.
4. Adds subagent spawning — the parent must declare task scope (a Subject `AttributeBag`).

It does **not** apply to:

- Pure utility code (sorting, parsing, formatting) — has no Subject/Resource.
- One-shot admin migrations — explicit `crossNamespace: true` bypass is documented in GKS.
- Existing GKS storage primitives — they are reused; this FRAME wraps them, does not replace them.

## Out of scope

- The four-tuple model **does not** prescribe a specific policy language. ADR-level decisions (e.g. YAML v1, Cedar later) live in `[[ADR--POLICY-AS-DATA-NOT-CODE]]` and siblings.
- This FRAME **does not** introduce domain-specific knowledge (medical, finance, legal). Domain knowledge enters via classifier plugins and policy packs; the core stays domain-agnostic.
- This FRAME **does not** replace `[[FRAME--MSP-ARCHITECTURE-V2]]`. The two coexist: v2 defines the three-layer stack; UCF defines how decisions flow inside it.

## Composition with existing MSP frames

- **MSP architecture v2** continues to own the cognitive ↔ Memory OS ↔ knowledge boundary. UCF inserts itself at the **Memory OS** layer: every entry point in MSP (Express, MCP, facade) becomes a Policy Enforcement Point (PEP).
- **Cognitive Layer facade** (`[[CONCEPT--COGNITIVE-LAYER-FACADE]]`) is the public entry. Its `recall`, `remember`, `runTask`, etc. accept Subject + Action — the four-tuple flows through unchanged.
- **Knowledge layers v2** (`[[CONCEPT--KNOWLEDGE-LAYERS-V2]]`) — atomic / vector / episodic / obsidian — remain the storage substrate. UCF adds attribute-bag metadata on top; it does not change the layer count.

## Decisions tracked in §0 of the spec

| Decision id | Topic | Result |
|---|---|---|
| D-1 | Policy language v1 | YAML + minimal operators (~200 LOC) — see `[[ADR--POLICY-AS-DATA-NOT-CODE]]` |
| D-2 | Resolution tier count (MVP) | 2-tier: FULL + MENTION + `expand()`; 4-tier deferred to Phase 4 — see `[[ADR--RESOLUTION-TIER-COUNT]]` |
| D-7 | Default policy posture | `default-permit` + shadow log in Phase 1; tighten per-endpoint from Phase 3 — see `[[ADR--DEFAULT-POLICY-POSTURE]]` |
| D-8 | AttributeBag storage | Atom metadata / frontmatter; GKS `Namespace` untouched — see `[[ADR--BRING-YOUR-OWN-ATTRIBUTES]]` |

Open questions remaining are tracked in spec §14 with working assumptions so atom extraction is unblocked.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0–§2, §10 — narrative SSOT.
- `gks/frame/[[FRAME--MSP-ARCHITECTURE-V2]].md` — parent architecture.
- `gks/concept/[[CONCEPT--COGNITIVE-LAYER-FACADE]].md` — the entry point that surfaces this FRAME.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]
- [[CONCEPT--ATTRIBUTE-BAG-MODEL]]
- [[CONCEPT--NAMESPACE-VAULT-BRAIN]]

