---
id: BLUEPRINT--MSP-MASTER-COMPOSE-LOADER
phase: 3
type: blueprint
status: draft
tier: genesis
source_type: axiomatic
vault_id: default
title: msp master compose CLI — sector-aware Master loader with multi-tiered triggers
tags: &a1
  - msp
  - master
  - loader
  - cli
  - blueprint
  - trigger-eval
crosslinks: &a2
  references:
    - ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS
    - CONCEPT--MASTER-PRIORITY-SECTORS
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - MASTER--MSP-DOC-TO-CODE
linked_symbols: &a3
  - file: packages/msp/src/master/compose-cli.ts
  - file: packages/msp/src/master/trigger-eval.ts
  - file: packages/msp/package.json
  - file: packages/msp/test/master/trigger-eval.test.ts
  - file: packages/msp/test/master/compose-cli.test.ts
created_at: 2026-05-17T02:50:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--MSP-MASTER-COMPOSE-LOADER
  phase: 3
  type: blueprint
  status: draft
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: msp master compose CLI — sector-aware Master loader with multi-tiered
    triggers
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-17T02:50:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--MSP-MASTER-COMPOSE-LOADER
    phase: 3
    type: blueprint
    status: draft
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: msp master compose CLI — sector-aware Master loader with multi-tiered
      triggers
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-17T02:50:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--MSP-MASTER-COMPOSE-LOADER
      phase: 3
      type: blueprint
      status: draft
      tier: genesis
      source_type: axiomatic
      vault_id: default
      title: msp master compose CLI — sector-aware Master loader with multi-tiered
        triggers
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-17T02:50:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# BLUEPRINT — msp master compose CLI loader

## 1. CLI signature

The `msp-master-compose` CLI provides a unified way to load Master atoms into an agent's context, respecting priority sectors and token budgets.

```bash
msp master compose \
  --sector=P0,P1,P2,P3 \
  [--namespace=<ns>] \
  [--root=<dir>] \
  [--out=<file>] \
  [--turn-context=<text>]
```

- `--sector`: Comma-separated list of sectors to include (default: `P0,P1`).
- `--namespace`: Filter Masters by namespace (default: all).
- `--root`: Project root directory (default: current working directory).
- `--out`: Output file path. If omitted, prints to stdout.
- `--turn-context`: The text of the current user turn and recent agent output, used for trigger evaluation in sectors P1–P3.

## 2. Multi-tiered trigger evaluation engine

The loader evaluates whether a Master's body should be loaded based on the `--turn-context` using a three-tiered approach.

### Tier 1: Keyword/Regex (Cheapest)

- **Logic**: Substring or regex match against `trigger.keywords[]`.
- **Scope**: User input + immediate previous agent output.
- **Short-circuit**: If match found, return `LOAD_BODY`.

### Tier 2: Semantic Context Match

- **Logic**: Check if the current environment matches `trigger.context[]` (e.g., active branch name, paths of modified files, specific tool failure types).
- **Short-circuit**: If match found, return `LOAD_BODY`.

### Tier 3: LLM Relevance Call (Expensive)

- **Logic**: Send a one-shot prompt to the LLM containing the Master ID, summary, and `trigger.llm_check` prompt, asking for a boolean relevance check against the conversation state.
- **Usage**: Only if Tiers 1 and 2 are inconclusive and the Master is in a requested sector.
- **Feature Flag**: Guarded by a feature flag/environment variable to prevent unexpected costs.

### Evaluation Order

1. Check Tier 1. If match, STOP (Load).
2. Check Tier 2. If match, STOP (Load).
3. Check Tier 3 (if enabled). If match, STOP (Load).
4. Otherwise, STOP (Index only).

## 3. Output format

The output is a single concatenated block designed for system-prompt injection.

### P0: Foundation Sector

- **Inclusion**: Always included if P0 is in `--sector`.
- **Content**: Full body but truncated to fit budget.
- **Sections**: Include `## Intent` and `## Directives`. **Skip** `## Why`, `## Apply when`, and `## Conflicts with`.
- **Header**: `# MASTER — <ID> (<path>)`

### P1: High-Priority Index

- **Index**: Always included if P1 is in `--sector`.
- **Body**: Included ONLY on trigger match.
- **Format (Index)**: `- <ID>: <1-line Intent summary> (→ <path>)`
- **Format (Body)**: Same as P0 (Intent + Directives).

### P2–P3: Contextual Indices

- **Index**: Included ONLY if a trigger (Tier 1 or 2) matches the `--turn-context`.
- **Body**: Included only if Tier 3 or a very strong Tier 2 match occurs.
- **Format**: Same as P1.

## 4. Token budget enforcement

The loader must strictly enforce per-sector caps defined in `[[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]`.

| Sector | Preamble Type | Sector Cap |
|---|---|---|
| P0 | Partial Body (Intent+Directives) | soft ~600 tokens |
| P1 | Index Entry | soft ~600 tokens |
| P2 | Index Entry | soft ~900 tokens |

- **Rejection**: If the total token count for a sector exceeds the cap, the loader must exit with error `1` and list the offending Masters.
- **Estimation**: Use a simple character-count heuristic (chars / 4) for fast validation, or a tiktoken-compatible library for precision.

## 5. Implementation location

- **Main CLI Entry**: `packages/msp/src/master/compose-cli.ts`
- **Trigger Evaluator**: `packages/msp/src/master/trigger-eval.ts`
- **Bin Entry**: Add `"msp-master-compose": "./dist/master/compose-cli.js"` to `packages/msp/package.json`.

## 6. Integration points

- **Master Atom Source**: Reads from `gks/master/*.md`.
- **Lazy Constituents**: The `constituents:` field is NOT loaded into the prompt by default. It is resolved only if the agent explicitly requests a "deep dive" into a specific Master.
- **Compatibility**: Operates independently of `msp-master-propose`. Propose handles *creation*; Compose handles *loading*.

## 7. Test plan

- **Unit tests**: `packages/msp/test/master/trigger-eval.test.ts`
  - Verify Tier 1 regex/substring matching.
  - Verify Tier 2 environment context matching.
  - Mock Tier 3 LLM calls.
- **Integration tests**: `packages/msp/test/master/compose-cli.test.ts`
  - End-to-end runs with mock `gks/master/` directory.
  - Verify sector filtering (`--sector=P0`).
  - Verify output format for P0 (skipping Why/Apply when).
- **Snapshot tests**:
  - Assert character/token counts stay within caps for fixture data.

## 8. Phase plan (microtasks)

- **T1: Schema migration helper**
  - Script to back-fill `priority:` and `constituents:` for existing Masters.
  - Update `MASTER--ROOT-CAUSE-ANALYSIS`, `MASTER--MSP-DOC-TO-CODE`, and `MASTER--ATOM-CONTRADICTION-POLICY` to `priority: P0`.
- **T2: compose-cli.ts skeleton + sector filtering**
  - Implement basic CLI argument parsing.
  - Implement file scanning and sector-based filtering logic.
- **T3: Trigger evaluator (Tier 1+2)**
  - Implement Tier 1 (keywords) and Tier 2 (basic context like file extensions).
  - Stub Tier 3.
- **T4: Token budget enforcement**
  - Implement token counting and cap validation.
  - Fail fast on budget overrun.
- **T5: Tier 3 LLM call**
  - Implement LLM relevance check behind a feature flag.
  - Integrate with the project's LLM provider.
- **T6: Integration tests + AUDIT atom**
  - Complete test suite coverage.
  - Create `AUDIT--MSP-MASTER-COMPOSE-LOADER.md` on success.
