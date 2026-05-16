---
id: SPEC--GENESIS-BLOCK-MANIFEST
phase: 2
type: spec
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Genesis Block manifest — frontmatter contract for GENESIS-- atoms (v2.3+)
tags:
  - msp
  - spec
  - knowledge-block
  - manifest
  - taxonomy
  - foundation
crosslinks: {"references":["CONCEPT--TAXONOMY-V2-3","ADR--TAXONOMY-V2-3-MIGRATION","FRAMEWORK--AUTHORITY-MATRIX","CONCEPT--GENESIS-GRAPH-BACKEND"]}
created_at: 2026-05-13T13:14:43+07:00
---

# SPEC — Genesis Block manifest

## 1. Disambiguation — "Genesis Block" vs "Genesis Graph Backend"

This repo contains two distinct concepts that have been informally referred to as "Genesis Block". This SPEC names them and pins one of them down.

| Name | What it is | Where it lives |
|---|---|---|
| **Genesis Block** | A *composite knowledge unit* — a `GENESIS--<NAME>` manifest atom + the member atoms it aggregates (five-dimension core per EVA 4.0: Cognitive + Algo + Runbook + Concept + Params, plus optional Guard/Safety/Stack/Protocol/Mod/Spec). Specified by **this atom**. | `packages/{gks,msp}/gks/genesis/GENESIS--<NAME>.md` (post-v2.3) |
| **Genesis Graph Backend** | An embedded graph **database backend** with Cypher v0, JSONL append log, bi-temporal time-travel. Pure storage; persists `manifest.json` + `genesis-block.jsonl`. Specified by `[[CONCEPT--GENESIS-GRAPH-BACKEND]]`. | `packages/gks/src/memory/graph/genesis-graph.ts` |

These are orthogonal — a Genesis Block can be *stored in* a Genesis Graph Backend (or in `GraphStore`, or in `PgGraphBackend`), but the composite identity is independent of the storage layer. Where existing prose says "Genesis Block" in the v2.3 taxonomy sense (composite knowledge), read it as "Genesis Block".

## 2. The manifest atom

A **Block Manifest** is a `GENESIS--<NAME>.md` atom under `packages/<pkg>/gks/genesis/` that declares the membership and authority of one Genesis Block. The taxonomy v2.3 prefix table (`[[CONCEPT--TAXONOMY-V2-3]]`) reserves `GENESIS--` for exactly this purpose. (Note: the prior placeholder prefix `FRAME--` was retired in favour of `GENESIS--` because `FRAME--` collided visually with `FRAMEWORK--` — see `[[ADR--TAXONOMY-V2-3-MIGRATION]]` §"Follow-up rename".)

### 2.1 Frontmatter (validator-required, inherited from the atomic contract)

These are required by `msp/LLM_Contract/atomic_contract.yaml` `required_fields.default` for every atom and apply unchanged here:

```yaml
id: GENESIS--<SLUG>          # e.g. GENESIS--IDENTITY-ENGINE
phase: 0                     # Block Manifests sit at P0 — they are foundational
type: genesis                # use 'genesis' (Block Manifest); 'framework' is for governance
status: stable               # see §4 for status cascade rules
title: <Genesis Block name — Engine — short purpose>
created_at: <YYYY-MM-DDTHH:MM:SS+07:00>   # ICT, per repo timezone rule
```

Recommended but optional (warning-only):

```yaml
tier: genesis                # see tier-enum.ts; "genesis" is correct for foundational blocks
source_type: axiomatic       # or "learned" if the block emerged from analysis
vault_id: default
tags: [..., manifest, knowledge-block]
```

### 2.2 Block-specific fields (declared by this SPEC, descriptive — see §5)

```yaml
manifest_version: <semver>   # e.g. "1.0.0" — block version, independent of SPEC version

members:
  core:                                  # EVA 4.0 5-dimension model
    cognitive: [COGNITIVE--<...>, ...]   # ≥1 entry — mental framework / lens (= EVA "Frame::X")
    algo:      [ALGO--<...>, ...]        # ≥1 entry — executable logic       (= EVA "Algo::X")
    runbook:   [RUNBOOK--<...>, ...]     # ≥1 entry — procedural SOPs        (= EVA "Proto::X")
    concept:   [CONCEPT--<...>, ...]     # ≥1 entry — origin / purpose       (= EVA "Concept::X")
    params:    [PARAMS--<...>, ...]      # ≥1 entry — tunable values         (= EVA "Param::X")
  optional:
    guard:     [GUARD--<...>, ...]       # structural invariants (data-integrity)
    safety:    [SAFETY--<...>, ...]      # ethical / alignment rules
    stack:     [STACK--<...>, ...]       # technology inventory for execution
    protocol:  [PROTOCOL--<...>, ...]    # A2A / MCP / HTTP interface (if exposed)
    mod:       [MOD--<...>, ...]         # module manifest (boundary + public API)
    spec:      [SPEC--<...>, ...]        # data shape / wire format

daci:
  driver:       MOD--<...> | PERSONA--<...> | ENTITY--<...>   # single owner of decisions
  approver:     [<id>, ...]              # who can `status: stable` promote
  contributor:  [<id>, ...]              # who can propose edits
  informed:     [<id>, ...]              # who must be told of changes
```

The `crosslinks.references` field already exists in every atom; the SPEC does **not** add new top-level frontmatter keys without `members:`/`manifest_version:`/`daci:`. (These three keys are not yet enforced by the validator — see §5.)

## 3. Membership rules

### 3.1 Five-dimension core (per EVA 4.0 — mandatory)

Per `gks_genesis_knowledge_system_summary.md` §2.1 (the EVA 4.0 canonical structure), every Genesis Block must be definable in **five** orthogonal dimensions. These are the **promotion criterion**: a block needs ≥4 of the 5 dimensions filled before it becomes a candidate for Master Block promotion.

| Role | v2.3 prefix | EVA 4.0 source | Purpose |
|---|---|---|---|
| **Cognitive** | `COGNITIVE--` | `Frame::X` | The mental framework / lens the block reasons with (e.g. Erikson 8 stages, Ego Death, Ship of Theseus, Artificial Qualia) |
| **Algo** | `ALGO--` | `Algo::X` | The step-by-step executable logic (e.g. Simulated Annealing, MRF, Meta Learning Loop, Emotional Learning Loop) |
| **Runbook** | `RUNBOOK--` | `Proto::X` | The procedural how-to / SOP — when and how to apply the algo in real conversation |
| **Concept** | `CONCEPT--` | `Concept::X` | The "why" — origin reflection, self-consistency, ethical memory root |
| **Params** | `PARAMS--` | `Param::X` | Tunable values (e.g. paradox_acceptance_threshold, principled_honesty, self_correction_urgency) |

`GENESIS--` is the *manifest itself*; it is not "a sixth member". The five core roles are closed.

> **Naming note**: in EVA 4.0 vocabulary, "Frame" means *mental framework* (a lens / model for interpreting events) — that maps to `COGNITIVE--` in v2.3. The v2.3 prefix `FRAMEWORK--` is a different concept (governance / architectural framework, used at the engineering layer). Do not confuse them.

### 3.2 Optional extensions (conditional)

The following supplement the five-dimension core when the block has the relevant concern:

- `GUARD--<...>` — when the block has structural data invariants (id-match, schema constraints, non-null)
- `SAFETY--<...>` — when the block takes outbound action (file writes, API calls, messages)
- `STACK--<...>` — when the block executes code that requires a specific runtime / library
- `PROTOCOL--<...>` — when the block exposes an external surface (A2A/MCP, HTTP/JSON-RPC, FFI)
- `MOD--<...>` — when the block declares a module boundary with a public API
- `SPEC--<...>` — when the block consumes or produces structured data with a documented shape

### 3.3 Aggregation grammar

Each id listed under `members.*` must:

1. Match the canonical id regex (`^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$`)
2. Resolve to an existing atom (the `dangling-wikilink` rule already enforces this on `crosslinks.references`; the SPEC asks authors to mirror every `members.*` id into `crosslinks.references` until a dedicated PROTO is written — see §5)
3. Have a `type:` consistent with its role under `members.*` (e.g. an id in `members.core.algo` must have `type: algo`)

## 4. Authority and lifecycle

### 4.1 DACI

`daci:` follows the standard four roles (Driver, Approver, Contributor, Informed). Values are atom ids (`MOD--`, `PERSONA--`, `ENTITY--`) — not free text. This aligns the Block Manifest with `[[FRAMEWORK--AUTHORITY-MATRIX]]`: write authority for any in-block atom flows through the manifest's `daci.approver` set.

### 4.2 Status cascade

A Genesis Block's `status` is **derived**: `status(block) = min(status(member))` under the order `stub < raw < draft < active < stable`. `deprecated` / `superseded` propagate immediately (one deprecated member moves the block to `deprecated`).

Authors should set `status: draft` on a new manifest and only flip to `stable` after every member is `stable` and the validator's `--all` run passes.

### 4.3 Supersession

When a Genesis Block evolves materially (a member is swapped for a different atom, a new role is added), publish a new `GENESIS--<NAME>-V<N+1>` and supersede the old one via standard `crosslinks.supersedes` / `crosslinks.superseded_by` reciprocal links. `manifest_version` increments independently of supersession — patch/minor bumps stay on the same atom id; majors trigger a new manifest.

## 5. Validation tier — this SPEC is descriptive, not enforced

The validator currently checks frontmatter shape via `required-fields` + the atomic contract. **Nothing in this SPEC's §2.2 / §3 is machine-checked yet.** The `members:` / `manifest_version:` / `daci:` fields are descriptive contract that authors and reviewers obey by convention.

A follow-up `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]` will:

- Add `frame` Block Manifest to the validator's per-type required-fields list (require `members.core` with all 5 dimensions present)
- Walk `members.*` and verify each id resolves and has the expected `type:`
- Apply the §4.2 status cascade and fail if `status:` disagrees with `min(member status)`
- Apply the **4-of-5 promotion criterion**: if ≥4 of the 5 core dimensions are filled with `status: stable` atoms, the block is a candidate for Master Block promotion (per `gks_genesis_knowledge_system_summary.md` §2.1)

That PROTO is **not** part of this PR (scope: SPEC only).

## 6. Worked example (mock — not authored)

A future `[[GENESIS--IDENTITY-ENGINE]]` Genesis Block might look like:

```yaml
---
id: GENESIS--IDENTITY-ENGINE
phase: 0
type: genesis
status: draft
title: Identity Engine — passport-bound agent identity resolution
created_at: 2026-05-13T13:14:43+07:00
tier: genesis
source_type: axiomatic
vault_id: default
tags: [msp, knowledge-block, identity, manifest]
manifest_version: 0.1.0
members:
  core:
    cognitive: [COGNITIVE--EGO-DEATH-PASSPORT]
    algo:      [ALGO--IDENTITY-RESOLUTION]
    runbook:   [RUNBOOK--IDENTITY-MIGRATION]
    concept:   [CONCEPT--IDENTITY-LAYER]
    params:    [PARAMS--IDENTITY-PROFILE-DEFAULTS]
  optional:
    guard:     [GUARD--IDENTITY-SCHEMA, GUARD--PASSPORT-NONNULL]
    protocol:  [PROTOCOL--IDENTITY-API]
    stack:     [STACK--MSP-NODE-RUNTIME]
    safety:    [SAFETY--PII-REDACTION]
    mod:       [MOD--IDENTITY]
daci:
  driver:      MOD--IDENTITY
  approver:    [PERSONA--T3-ARCHITECT]
  contributor: [PERSONA--T2-IMPLEMENTER]
  informed:    [ENTITY--MSP-USERS]
crosslinks:
  references:
    - SPEC--GENESIS-BLOCK-MANIFEST
    - MOD--IDENTITY
    - ALGO--IDENTITY-RESOLUTION
    - COGNITIVE--EGO-DEATH-PASSPORT
    - CONCEPT--IDENTITY-LAYER
    - RUNBOOK--IDENTITY-MIGRATION
    - PARAMS--IDENTITY-PROFILE-DEFAULTS
    - GUARD--IDENTITY-SCHEMA
    - GUARD--PASSPORT-NONNULL
    - PROTOCOL--IDENTITY-API
    - STACK--MSP-NODE-RUNTIME
    - SAFETY--PII-REDACTION
---

# FRAME — Identity Engine

(Body explains what the engine does, links its members in narrative form,
and points readers at the SOPs and protocols.)
```

This atom is **not** created by this PR. Several of its members (`[[COGNITIVE--EGO-DEATH-PASSPORT]]`, `[[GUARD--IDENTITY-SCHEMA]]`, `[[STACK--MSP-NODE-RUNTIME]]`, etc.) do not exist yet and would need authoring first.

## 7. What this SPEC does not cover

- The runtime that *loads* a Genesis Block manifest and invokes its members in order — that's a future `[[BLUEPRINT--KNOWLEDGE-BLOCK-RUNTIME]]`.
- The contract for `COGNITIVE--`, `STACK--`, `SAFETY--` themselves (each is a separate frontmatter contract; one SPEC each, written when the first atom of that type is authored).
- Resonance Index calculation (mentioned in the v1.2 draft of the user's specification) — deferred to a `[[SPEC--RESONANCE-INDEX]]` atom.
- Storage-engine choice — whether a block's edges go into `GraphStore`, `PgGraphBackend`, or `GenesisGraphBackend` is orthogonal to the manifest shape. The manifest does not declare a backend.
