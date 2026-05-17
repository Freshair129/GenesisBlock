---
id: AUDIT--SYMBOLS-FRAMEWORK-AWARENESS
phase: 6
type: audit
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Audit results for framework-awareness recognizers
tags:
  - msp
  - symbol-graph
  - audit
  - nextjs
  - prisma
  - fastapi
crosslinks: {"references":["CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS","ADR--SYMBOLS-FRAMEWORK-AWARENESS","ALGO--SYMBOLS-FRAMEWORK-RECOGNITION","PROTO--SYMBOLS-FRAMEWORK-INVARIANTS"]}
created_at: 2026-05-12T22:30:00.000+08:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Framework-awareness recognizers

## Scope verified
- Implementation of 6 specialized framework recognizers:
  - `NextJsRecognizer`: App Router conventions.
  - `RoutesRecognizer`: Pages Router API + FastAPI.
  - `RuntimeTagRecognizer`: Client/Server directive detection.
  - `DataFetchingRecognizer`: Loader identification.
  - `OrmRecognizer`: Prisma/Drizzle entity extraction.
  - `McpToolRecognizer`: MCP tool registration discovery.
- Integration via `FrameworkRegistry`.
- Structural validation via `[[PROTO--SYMBOLS-FRAMEWORK-INVARIANTS]]`.

## Test results
- **Total Tests**: 21 cases across 8 test files.
- **Pass Rate**: 100% (21/21 passed).
- **Test Files**:
  - `test/symbols/framework/nextjs.test.ts`
  - `test/symbols/framework/routes.test.ts`
  - `test/symbols/framework/runtime-tag.test.ts`
  - `test/symbols/framework/data-fetching.test.ts`
  - `test/symbols/framework/orm.test.ts`
  - `test/symbols/framework/mcp-tools.test.ts`
  - `test/symbols/framework/registry.test.ts`
  - `test/validator/proto/framework-invariants.test.ts`

## Deviations
- Added `async` support to `DataFetchingRecognizer` to correctly detect asynchronous legacy loaders.
- Relaxed `OrmRecognizer` path matching to support Drizzle schemas without the `.schema.ts` suffix if located in a `db/schema` directory.
- Enabled `active` status support in `loader.ts` to allow gradual promotion of PROTOs.

## Follow-ups
- Refine line-number extraction for ORM entities (currently defaults to 1).
- Add support for Remix/SvelteKit routers in Phase 3.

## Connections
- [[CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS]]
- [[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]
- [[ALGO--SYMBOLS-FRAMEWORK-RECOGNITION]]

