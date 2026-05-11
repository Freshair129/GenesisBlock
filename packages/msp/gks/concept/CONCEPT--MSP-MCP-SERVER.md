---
id: CONCEPT--MSP-MCP-SERVER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP MCP server — expose MSP-specific tools via stdio MCP
tags:
  - msp
  - mcp
  - server
  - integration
crosslinks: {"references":["FEAT--MSP-VALIDATOR","FEAT--CODEGEN-MICROTASK-RUNNER","FEAT--MEMORY-SESSIONS-WRITER","FEAT--MEMORY-EPISODIC-WRITER","FEAT--MEMORY-BACKLINKS-INDEXER"]}
created_at: 2026-05-03T11:13:53.808Z
---

# CONCEPT — MSP MCP server

## Problem

`@freshair129/gks` ships `gks-mcp-server` with 12 tools (retain, recall, lookup, propose-inbound, verify-flow, etc.). MCP-aware clients (Claude Code, Cursor) can call those directly. But the MSP-specific surface — validator, codegen runner, sessions/episodic/backlinks writers — is only reachable via npm scripts. Agents have to shell out via Bash, lose typed I/O, and can't get tool-use telemetry.

## Hypothesis

A standalone `msp-mcp-server` binary that exposes MSP's specific tools — and **only** those — gives agents a clean, typed surface. Run it side-by-side with `gks-mcp-server` (per the GksV3 + GitNexus pattern in their README): the client merges the tool surfaces; agents see both `gks_*` and `msp_*` tools without conflict.

## Scope

In:
- Stdio MCP server using `@modelcontextprotocol/sdk`.
- 6 tools, all MSP-specific:
  - `msp_validate` — run validator on file(s) or `--all`
  - `msp_propose` — propose to inbound (handles phase 6 like the wrapper)
  - `msp_run_task` — execute a `T*.task.yaml` via codegen runner (mock SLM by default; pluggable by env)
  - `msp_session_append` — append a turn via the sessions writer
  - `msp_episode_append` — append an episode via the episodic writer
  - `msp_backlinks_rebuild` — rebuild + check the backlinks file
- Bin entry `msp-mcp-server` pointing at `dist/mcp/server.js`.
- Each tool has Zod-strict input schema (per gks-mcp-server's pattern).
- Tools delegate to existing TS modules — server is glue, not new logic.

Out:
- Re-exporting `gks_*` tools — defeats the purpose; users run both servers.
- Authentication / TLS — local stdio.
- Streaming responses — single round-trip per tool call.
- Long-running task lifecycle (the codegen runner is one-shot per call).

## Source

Closes P1 #5 (MCP server) from the M3 production-readiness backlog. Last large item before MSP can be considered "feature complete" relative to the spec.
