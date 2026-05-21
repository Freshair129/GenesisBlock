---
id: BLUEPRINT--NOTION-IMPORT-TOOLING
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Notion Import Tooling Implementation Plan
tags: [msp, migration, notion, plan, m9d]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--NOTION-IMPORT-TOOLING
    - CONCEPT--MSP-ROADMAP
linked_symbols:
  - file: packages/msp/src/migration/notion/cli.ts
  - file: packages/msp/src/migration/notion/client.ts
  - file: packages/msp/src/migration/notion/mapper.ts
created_at: 2026-05-21T23:15:00+07:00
---

# BLUEPRINT — Notion Import Tooling

## 1. Goal

Implement the technical machinery to fetch, convert, and map Notion pages/databases into GKS atoms, using the official Notion API and established transformation libraries.

## 2. Implementation Steps

### T1: Dependency Setup

- Install official Notion client and transformation helpers in the `msp` package.
- Command: `npm install @notionhq/client notion-to-md --workspace=packages/msp`.

### T2: Notion Client Wrapper (`packages/msp/src/migration/notion/client.ts`)

- Implement a wrapper that initializes the `@notionhq/client`.
- Logic to fetch a database's pages or a single page's blocks recursively.
- Handle rate limiting and pagination.

### T3: Markdown Transformer (`packages/msp/src/migration/notion/transform.ts`)

- Use `notion-to-md` to convert Notion block objects into Markdown.
- Add custom handlers for:
  - **Links:** Attempt to resolve internal Notion page IDs to existing or pending GKS IDs.
  - **Properties:** Map Notion properties (e.g., "Status", "Tags", "ID") to the GKS YAML frontmatter template.

### T4: Mapping Heuristics (`packages/msp/src/migration/notion/mapper.ts`)

- Implement logic to determine the GKS `type` and `id` from Notion metadata.
- Default to `CONCEPT` if type cannot be inferred.
- Clean and normalize titles into screaming-kebab-case slugs.

### T5: CLI Implementation (`packages/msp/src/migration/notion/cli.ts`)

- Implement `msp-migrate notion` command using `node:util.parseArgs`.
- Options:
  - `--token`: Notion API token (or read from env `NOTION_TOKEN`).
  - `--database-id`: The UUID of the source database.
  - `--out-dir`: Destination folder (defaults to `gks/inbound/`).
- Output: Logs for each successfully converted page.

## 3. Verification Plan

### 3.1 Mock Integration Test

- Use the `mock` provider in the Notion client.
- Feed a set of synthetic Notion block objects (containing tables, code blocks, and callouts).
- Verify that the resulting `.md` file in `gks/inbound/` is correctly formatted and passes `msp:validate`.

### 3.2 Manual Acceptance (SIT)

- Connect to a test Notion workspace.
- Import a "Decision" database.
- Verify that the resulting atoms in `gks/inbound/` correctly preserve the "Status" and "Tags" from Notion.
