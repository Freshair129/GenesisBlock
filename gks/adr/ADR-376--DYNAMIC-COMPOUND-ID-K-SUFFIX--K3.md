---
id: ADR-376--DYNAMIC-COMPOUND-ID-K-SUFFIX--K3
knowledgeId: DYNAMIC-COMPOUND-ID-K-SUFFIX
phase: 2
type: adr
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Dynamic Compound ID Suffix with Phase-Layer K Numbering
aliases:
  - ADR
cluster: implementation_flow
role: Architecture decision record
tags:
  - msp
crosslinks: {}
created_at: 2026-05-17T09:50:53.301+07:00
---

# ADR — Dynamic Compound ID Suffix with Phase-Layer K Numbering

## Context

As the Genesis Knowledge System (GKS) expands, the compound primary key identifiers must provide rich metadata, prevent prefix collisions, and maintain a highly readable structure. 

Initially, GKS atoms used a simple two-part or three-part identifier scheme:
*   Standard legacy format: `TYPE--SLUG` (e.g. `CONCEPT--USAGE-ROLLUPS`)
*   Sequence sequence format: `TYPE-seq--SLUG` (e.g. `CONCEPT-concept-01--USAGE-ROLLUPS`)

However, as the taxonomy grew to support new process types, prefix collisions became a serious issue:
*   `ADR` (Architecture Decision), `ALGO` (Algorithm), and `API` (OpenAPI Hub) all share the letter **`A`**.
*   `FEAT` (Feature spec) and `FLOW` (UI/Data flow) share the letter **`F`**.
*   `ENTITY` (Data schema) and `ENDPOINT` (API Method) share the letter **`E`**.

If we simply used the first character of the type name (`type.charAt(0)`), their sequential sequence numbers (e.g. `1-A1`, `1-A2`) would collide. Furthermore, having sequential codes directly inside the sequence block reduced ID readability and made it hard to visually rank files by their structural development stages (Phases P0 to P6).

## Decision

We decide to upgrade the compound ID scheme to a universal, dynamic phase-tailed standard:
`{aliases}-{atom_counter}--{knowledgeId}--K{atomtype_counter}`

Where:
1.  **`{aliases}`**: The primary uppercase type prefix of the GKS atom (e.g. `IDEA`, `CONCEPT`, `ADR`).
2.  **`{atom_counter}`** (`atomId`): The total global chronological count of all markdown files in the GKS vault recursively (e.g. `376`).
3.  **`{knowledgeId}`** (`slug`): The semantic screaming-kebab-case identifier of the subject (e.g. `DYNAMIC-COMPOUND-ID-K-SUFFIX`).
4.  **`{atomtype_counter}`**: The 1-based index representing the Phase Stage (derived from the custom `counter` field in `atom_registry.yaml`, falling back to `config.phase + 1` if undefined). For example, `K1` for Phase 0 sparks, `K2` for Phase 1 concepts, `K3` for Phase 2 spec sheets like ADRs and Features.

### Global Defaults Integration
The GKS scaffolder is updated to treat this format as the universal fallback standard. Types that do not explicitly override their template inside `atom_registry.yaml` automatically inherit this phase-tailed format from the centralized `schema_spec` at the head of `atom_registry.yaml`.

## Consequences

1.  **Zero Collisions**: Since each type phase is explicitly mapped to a unique `K` counter representing the build pipeline phase (Phase 0 $\rightarrow$ `K1`, Phase 1 $\rightarrow$ `K2`, etc.) and paired with a global chronological number, there are absolutely zero namespace or sequence number overlaps.
2.  **Clean Separation of Concerns**: The ID is beautifully partitioned. The front block provides clean chronological indexing (`ADR-376`), the middle block provides semantic description (`--DYNAMIC-COMPOUND-ID-K-SUFFIX`), and the tail block stamps structural hierarchy (`--K3`).
3.  **Flawless Backwards Compatibility**: The indexing and validation layers split identifiers by `-` instead of `--` to extract prefixes, and allow the optional `(?:--K\d+)?` suffix, supporting both legacy and compound structures concurrently:
    *   Legacy: `CONCEPT--USAGE-ROLLUPS` $\rightarrow$ Valid
    *   Compound: `CONCEPT-concept-01--USAGE-ROLLUPS` $\rightarrow$ Valid
    *   Phase-Tailed: `ADR-376--DYNAMIC-COMPOUND-ID-K-SUFFIX--K3` $\rightarrow$ Valid

## Alternatives considered

*   **Custom Abbreviation Mapping (`seq_char`)**: Allowing custom type prefixes (e.g. `AD` for ADR, `AG` for ALGO, `AP` for API). While highly customizable, this introduces manual taxonomy overhead and is less structured than utilizing the existing `phase` hierarchy of GKS.

## Source

*   [atom_registry.yaml](file:///c:/Users/freshair/cognitive_system/atom_registry.yaml)
*   [msp-atom.ts](file:///c:/Users/freshair/cognitive_system/scripts/msp/msp-atom.ts)
