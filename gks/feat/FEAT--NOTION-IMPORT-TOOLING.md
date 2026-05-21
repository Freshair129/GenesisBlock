---
id: FEAT--NOTION-IMPORT-TOOLING
phase: 2
type: feat
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: FEAT — Notion Import Tooling — migrating legacy knowledge into GKS
tags: [msp, migration, notion, automation, m9d]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - CONCEPT--KNOWLEDGE-LAYERS-V2
created_at: 2026-05-21T23:00:00+07:00
---

# FEAT — Notion Import Tooling

## 1. Summary

The Notion Import Tooling is a migration utility designed to ingest legacy documentation, meeting notes, and project specs from Notion and convert them into standards-compliant GKS atoms. It provides an automated bridge for organizations moving from centralized SaaS wikis to a distributed, agent-agentic knowledge system.

## 2. Motivation

Many projects starting with GKS already have significant knowledge stored in Notion. Manually copying and reformatting hundreds of pages into Markdown with the correct GKS frontmatter is a prohibitive barrier to adoption. M9d provides a CLI-based path to rapidly bulk-import this knowledge, preserving metadata and relationships while enforcing GKS governance standards.

## 3. Requirements

### 3.1 Authentication & Connectivity

- Support Notion API integration via an Internal Integration Token (`NOTION_TOKEN`).
- Support targeting specific databases or individual page IDs.

### 3.2 Content Transformation

- Convert Notion blocks (paragraphs, headings, lists, code blocks, images) into standard GitHub-Flavored Markdown.
- Identify and preserve internal page references, converting them into GKS `[[wikilinks]]` where possible.
- Handle Notion properties (Select, Multi-select, Date) and map them to GKS frontmatter fields (tags, created_at, status).

### 3.3 Atomic Mapping

- Provide a heuristic to map Notion database types to GKS types (e.g., a "Decisions" DB in Notion maps to `ADR--*`).
- Automatically generate GKS-compliant IDs based on the Notion page title or a dedicated property.

### 3.4 CLI Interface

- Provide a command `msp-migrate notion [--database <id>] [--page <id>] [--out <dir>]`.
- Support a `--dry-run` flag to preview the generated atoms before writing to disk.
- Support a `--mapping <file.json>` to define custom Notion-to-GKS field mappings.

## 4. Acceptance Criteria

- [ ] A CLI command exists to fetch and convert Notion content.
- [ ] Complex Notion pages (containing nested lists and code blocks) are accurately represented in the resulting Markdown.
- [ ] Notion database properties are successfully converted into YAML frontmatter.
- [ ] The imported atoms pass `msp:validate` with minimal manual correction.

## 5. Connections

- `[[CONCEPT--MSP-ROADMAP]]` §3 M9d.
- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` — defines the structure imported data must adhere to.
