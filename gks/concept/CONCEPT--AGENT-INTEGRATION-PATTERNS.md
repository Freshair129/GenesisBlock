---
id: CONCEPT--AGENT-INTEGRATION-PATTERNS
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Agent integration patterns — three shapes (MCP-native / MCP-bridged / shell-wrapped) honoring one contract
tags:
  - msp
  - agent-agnostic
  - integration
  - mcp
  - patterns
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--AGENT-AGNOSTIC","CONCEPT--NAMED-PROJECT-REGISTRY","ADR--GLOBAL-VS-WORKSPACE"]}
created_at: 2026-05-10T07:00:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — Agent integration patterns

## Problem

`[[CONCEPT--AGENT-AGNOSTIC]]` declared MSP must plug into any cognitive-layer client. That declaration is necessary but not sufficient — without a single, predictable wire-in pattern, every client team reinvents the integration: different env vars, different config file shapes, different project-resolution heuristics, different identity merge order.

The result is fragmentation that defeats the agent-agnostic intent. A user moving from Claude Code to Gemini CLI shouldn't have to relearn MSP. A clinic project shared across Antigravity (human-supervised) and a headless CI bot shouldn't fork its config.

We need a **contract** every cognitive-layer client honors, plus an enumeration of the **shapes** that contract takes in practice.

## The contract

Every MSP-integrated client must:

1. **Launch `msp-mcp-server` as an MCP stdio server.** The bin is the only supported surface. No client should re-implement MSP tools by parsing `.brain/msp/projects/<ns>/` directly. The bin is published from this repo's `package.json` (see `bin.msp-mcp-server`); installation is via `npm install -g msp` or vendored.

2. **Honor the project-resolution chain** from `[[CONCEPT--NAMED-PROJECT-REGISTRY]]`:
   - `--project=<name>` CLI flag / MCP tool argument (highest priority)
   - `MSP_PROJECT=<name>` env var
   - `.mspconfig` in cwd or any ancestor
   - Fallback: `default`

   Clients pass through whichever of these the user supplies; they do not inject their own override.

3. **Merge global identity + workspace override** per `[[ADR--GLOBAL-VS-WORKSPACE]]`:
   - Read `~/.msp/identity.json` (or `$MSP_HOME/identity.json` if set) as the base.
   - Shallow-merge `./.brain/msp/projects/<ns>/identity.override.json` if present.
   - Result is the resolved `Identity` object the agent sees.

   Clients never reach into either file directly — the `msp_identity_get` MCP tool is the single read path.

4. **Pass `MSP_HOME` through unchanged** if the user sets it. Tests, CI, and multi-account setups depend on this; clients that strip env vars break the contract.

That is the entire contract. Everything else (slash commands, permission UX, idle timeouts, marketplace branding) is client-specific and outside MSP's scope.

## Three integration shapes

The contract is uniform; the **shape** of the integration varies by what the client supports natively.

### Shape 1 — MCP-native

The client speaks MCP directly and has a config file (JSON or TOML) listing MCP servers. Wiring is one block:

```jsonc
{ "mcpServers": { "msp": { "command": "msp-mcp-server" } } }
```

Clients in this shape: Claude Code, Gemini CLI, Antigravity, Cursor, Codex / ChatGPT custom agents.

Adding `MSP_PROJECT` is one line in the same block (`"env": { "MSP_PROJECT": "clinic" }` or equivalent).

### Shape 2 — MCP-bridged

The client doesn't have a first-class MCP config but has an extension / SDK that can spawn an MCP server. The integration is a small adapter (~10–20 LoC) that calls `@modelcontextprotocol/sdk`'s stdio transport and wires the resulting tools into the client's tool registry.

Clients in this shape: custom Python agents using `mcp` SDK, custom TypeScript agents using `@modelcontextprotocol/sdk`, agents built on frameworks (LangChain, LlamaIndex) where MCP support is a plugin.

The contract is identical — the adapter is a thin transport layer, not a translator.

### Shape 3 — shell-wrapped

The client can't speak MCP at all (legacy CI bot, headless script, no SDK in the host language). The integration is a `bash`/`sh` wrapper that calls `npx tsx src/...` or vendored CLI bins (`msp-validate`, `msp-backlinks`, `msp-graph`) via subprocess.

Clients in this shape: GitHub Actions running validator gates, headless agents that only need write-side operations (e.g. propose-candidate-then-exit), pipelines whose host language has no MCP SDK.

This shape gives up the **MCP-native** features (live tool discovery, structured I/O via JSON-RPC), but preserves the **contract** — env vars and project resolution still apply because the underlying CLIs read them.

## What's portable

- The single bin name (`msp-mcp-server`) and its CLI surface (`--root=<path>` or env-driven).
- The 4 contract rules above.
- Project resolution (CLI → env → `.mspconfig` → `default`).
- Global identity merge order.
- The `MSP_HOME` escape hatch.

A user with a working integration in client A can copy the relevant 3–5 lines into client B's config and it works. That is the agent-agnostic claim made operational.

## What's NOT portable

- **Slash command UX.** Claude Code's `/msp-recall`, Cursor's `@msp`, Gemini CLI's `/recall` are each client conventions. MSP exposes tools; how the client surfaces them as commands is the client's UX decision.
- **MCP server permission models.** Claude Code prompts on first call; Antigravity allow-lists by tool name; Cursor uses workspace settings. MSP cannot control any of these — clients gate at their own layer.
- **Idle-timeout / process lifecycle.** Some clients keep the MCP server alive for the session; others spawn per-call. MSP's stdio server is stateless across launches (state is on disk), so both work — but observable latency differs.
- **Telemetry / tool-use logging.** Each client has its own audit log. MSP's per-project sessions log is independent and authoritative for MSP state; clients may or may not surface it.
- **OS-level packaging.** Where `msp-mcp-server` lives on disk (`/usr/local/bin`, `~/.npm/...`, vendored) is an install decision. The integration patterns assume the bin is on `PATH` or absolutely-pathed in the config.

## Why three shapes (not one universal SDK)

Two reasons:

1. **The MCP ecosystem is the universal SDK.** Anthropic's `@modelcontextprotocol/sdk` already provides Python, TypeScript, and others. MSP doesn't ship its own SDK because there's nothing to add — the protocol is the contract.
2. **Shell-wrapped is unavoidable.** CI runners and constrained environments can't always pull an MCP SDK. The CLI bins exist for exactly this case. They are not a parallel API surface — they read the same `.brain/msp/projects/<ns>/` files the MCP server does.

## Trade-offs

**Positive**
- Documented integration is copy-paste, not bespoke per client.
- New cognitive-layer clients can adopt MSP in minutes if they speak MCP.
- The contract is small enough to memorize (4 rules), large enough to actually work.

**Negative**
- Three shapes mean three slightly-different debugging paths when something breaks ("is it the config? the env var? the bin not on PATH?"). Mitigated by the verification step in `docs/AGENT-INTEGRATION.md` (call `msp_recall "test"` once after wire-up).
- Client-specific UX (slash commands) means the user-visible MSP surface still varies; we accept that as out-of-scope.

## Source

Phase C of the architecture-doc cleanup (2026-05-10). Operationalises `[[CONCEPT--AGENT-AGNOSTIC]]` by enumerating the contract every cognitive-layer client honors and the three shapes that contract takes. Concrete wiring snippets live in `docs/AGENT-INTEGRATION.md`. See `[[AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

