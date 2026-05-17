---
id: CONCEPT--CODEGEN-MICROTASK-RUNNER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen microtask runner — execute T*.task.yaml under the codegen contract
tags: &a1
  - msp
  - codegen
  - runner
  - microtask
crosslinks: &a2
  references:
    - CONCEPT--CODEGEN-MICROTASK-CONTRACT
    - ADR--CODEGEN-RETRY-POLICY
created_at: 2026-05-03T14:16:36.203+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--CODEGEN-MICROTASK-RUNNER
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Codegen microtask runner — execute T*.task.yaml under the codegen contract
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:16:36.203+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--CODEGEN-MICROTASK-RUNNER
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Codegen microtask runner — execute T*.task.yaml under the codegen contract
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:16:36.203+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — codegen microtask runner

## Problem

`gks new-feature` produces `T*.task.yaml` files under `.brain/<ns>/tasks/<feature>/`. They have a parent BLUEPRINT, geography, prompt, and acceptance list. But nothing **runs** them. The doc-to-code chain dead-ends at P4 with a folder of nicely-formatted YAMLs and no source code.

## Hypothesis

A runner that loads a task YAML, calls the configured SLM, applies the codegen contract checks, runs acceptance tests, and retries (or escalates) per `[[ADR--CODEGEN-RETRY-POLICY]]` will close the gap from P4 to P5 mechanically. Same input → same output (modulo SLM nondeterminism), and every run is auditable.

## Scope

In:
- Load `T*.task.yaml`, validate shape (id, parent_blueprint, prompt, acceptance, geography).
- Invoke a pluggable SLM (Qwen 2.5 Coder default; configurable per-task).
- Apply post-processing + forbidden-pattern checks (per the codegen ADRs).
- Write candidate code to a sandbox path; run acceptance test.
- Retry up to 3 with feedback; escalate to Gemini → Opus on continued failure.
- Emit one audit row per run (`gks/devlog/MSP-ACT-<id>.md`).

Out:
- BLUEPRINT generation (that's a separate concern at P3).
- Multi-task orchestration (one task per invocation; orchestrator above sequences them).
- Code review of accepted output (humans + CI).

## Source

Implements `[[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]`. Spec §5 + §11 (Tooling).
