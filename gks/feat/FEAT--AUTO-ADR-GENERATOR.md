---
id: FEAT--AUTO-ADR-GENERATOR
phase: 2
type: feat
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: FEAT — Auto-ADR Generator — reducing doc-first friction via automation
tags: [msp, governance, automation, adr, m9e]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - ADR--DOC-TO-CODE-ENFORCEMENT
created_at: 2026-05-18T13:00:00+07:00
---

# FEAT — Auto-ADR Generator

## 1. Summary

The Auto-ADR Generator is a productivity tool designed to minimize the overhead of the mandatory "Doc-Before-Code" workflow. It enables agents and human contributors to automatically draft Architecture Decision Records (ADRs) based on staged code changes or natural language descriptions, ensuring that governance documentation keeps pace with implementation without manual drudgery.

## 2. Motivation

The `[[ADR--DOC-TO-CODE-ENFORCEMENT]]` policy is critical for long-term project health, but it introduces friction. Creating a high-quality ADR manually takes 15–30 minutes, which can tempt developers to skip documentation or produce low-quality "placeholders." By automating the drafting process (reducing it to ~30 seconds), we maintain strict governance while maximizing developer velocity.

## 3. Requirements

### 3.1 Code Change Analysis

- The tool must be able to analyze `git diff` of staged changes.
- It should identify affected modules, functions, and symbols.
- It should infer the *intent* of the change from code comments, commit messages, or a brief user hint.

### 3.2 Automated Drafting

- Generate a standards-compliant Markdown ADR file in `gks/adr/`.
- Automatically populate the YAML frontmatter (ID generation, timestamp, tags, aliases, tier).
- Draft the core ADR sections:
  - **Context:** Why is this change being made? What was the previous state?
  - **Decision:** What is the specific technical choice?
  - **Consequences:** What are the pros/cons of this choice?
- Support multiple SLM backends (Ollama, Gemini, etc.) for the drafting logic.

### 3.3 CLI Interface

- Provide a command `msp-adr draft [--hint "<text>"] [--staged]`.
- `--staged`: Draft based on current git staged changes.
- `--hint`: Provide a short sentence to guide the LLM's understanding of the decision.
- Allow outputting to stdout or directly creating the file.

### 3.4 Governance Compatibility

- The generated ADR must be marked with `status: draft` or `status: raw` to signal that it requires human/senior review.
- Must pass `msp:validate` immediately upon creation.

## 4. Acceptance Criteria

- [ ] A CLI command exists to generate an ADR from staged git changes.
- [ ] The generated ADR includes valid frontmatter and all required sections.
- [ ] The generated ID follows the universal phase-tailed standard if applicable.
- [ ] The tool successfully uses the configured SLM provider for content generation.

## 5. Connections

- `[[CONCEPT--MSP-ROADMAP]]` §3 M9e.
- `[[ADR--DOC-TO-CODE-ENFORCEMENT]]` — the policy this feature supports.
