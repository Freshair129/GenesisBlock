---
id: BLUEPRINT--AUTO-ADR-GENERATOR
phase: 3
type: blueprint
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: BLUEPRINT — Auto-ADR Generator Implementation Plan
tags: [msp, governance, automation, adr, plan, m9e]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
linked_symbols:
  - file: packages/msp/src/utils/git.ts
  - file: packages/msp/src/generator/adr-engine.ts
  - file: packages/msp/src/cli/adr-cli.ts
crosslinks:
  references:
    - FEAT--AUTO-ADR-GENERATOR
    - CONCEPT--MSP-ROADMAP
created_at: 2026-05-18T13:15:00+07:00
---

# BLUEPRINT — Auto-ADR Generator

## 1. Goal

Implement the technical machinery to automatically draft Architecture Decision Records (ADRs) by analyzing code changes and utilizing LLMs for content synthesis, reducing documentation overhead.

## 2. Implementation Steps

### T1: Git Integration (`packages/msp/src/utils/git.ts`)

- Implement a helper to extract the `git diff` of currently staged files.
- Command: `git diff --cached`.
- Include a list of staged filenames to help with context.

### T2: ADR Content Engine (`packages/msp/src/generator/adr-engine.ts`)

- Create a prompt template for the LLM that includes:
  - The `git diff`.
  - User-provided hint (optional).
  - Current codebase context (relevant symbols).
  - Specific ADR sections (Context, Decision, Consequences).
- Instructions to return valid Markdown with appropriate sections.
- Use `createSlmClient` to interface with the configured provider.

### T3: CLI Implementation (`packages/msp/src/generator/adr-cli.ts`)

- Implement `msp-adr draft` command.
- Logic:
    1. Fetch staged diff (T1).
    2. Prompt user for a hint if none provided (optional).
    3. Call the ADR Engine (T2) to get the draft content.
    4. Generate a compliant `atom_id` using the universal phase-tailed standard.
    5. Construct the file with full frontmatter.
    6. Write to `gks/adr/`.
    7. Provide success message with the new file path.

### T4: ID and Metadata Automation

- Reuse ID generation logic from `atom-creator` skill or implement a shared utility.
- Ensure `created_at` uses ICT timestamp (+07:00).
- Auto-tag based on affected file paths (e.g., if `packages/gks` is touched, add `gks` tag).

### T5: Integration and Build

- Register `msp-adr` in `packages/msp/package.json` under `bin`.
- Add `npm run msp:adr` script.

## 3. Verification Plan

### 3.1 Unit Tests

- Create `packages/msp/test/generator/adr-engine.test.ts`.
- Mock git diff and LLM response.
- Verify section extraction and frontmatter generation.

### 3.2 Manual Acceptance

- Stage a small code change.
- Run `npm run msp:adr draft -- --staged --hint "Refactoring the logger to use a singleton"`.
- Verify the generated file in `gks/adr/` passes `msp:validate`.
