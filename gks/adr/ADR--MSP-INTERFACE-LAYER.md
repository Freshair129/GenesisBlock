---
id: ADR--MSP-INTERFACE-LAYER
phase: 2
type: adr
status: draft
vault_id: default
tier: genesis
source_type: documented_source
title: MSP Interface Layer — Hexagonal seam for inbound adapters (Smart Proxy Pattern)
tags:
  - msp
  - architecture
  - hexagonal
  - gateway
  - interface
  - adapter
  - decision
crosslinks: {"implements":["FRAMEWORK--MSP-ARCHITECTURE-V2"],"references":["ADR--MONOREPO-STRUCTURE","ADR--MSP-MCP-SERVER","CONCEPT--AGENT-AGNOSTIC"]}
created_at: 2026-05-12T03:30:00.000+07:00
---

# ADR — MSP Interface Layer (Smart Proxy Pattern)

## Context

`[[CONCEPT--AGENT-AGNOSTIC]]` declared MSP as a "passport-orchestrator that travels with any cognitive-layer client." Today MSP already exposes one inbound surface — the MCP server at `src/mcp/` — and one outbound surface — the GKS wrapper at `src/memory/`. The remaining concerns (codegen, validator, identity, session handling) live as flat siblings in `src/`.

As MSP starts being adopted as a Gateway by external systems (OpenClaw-style chatbots receiving Slack/Discord webhooks, REST APIs for web UIs, agent-to-agent protocols), we face a design decision: **should new interfaces (Slack, Discord, REST) live inside MSP, beside it as separate services, or somewhere else?**

Three concrete forces:

1. **Hexagonal pressure** — orchestration logic (recall → think → act → retain → reflect loop) must not depend on which transport delivered the request. Mixing `slack.WebClient` calls into the orchestrator core makes the loop untestable and platform-locked.
2. **Operational pressure** — early-stage deployment wants one service to run (not five). High-traffic / high-security deployments later want Gateway split out to DMZ.
3. **Existing seam** — `src/mcp/` already shows the right shape: a thin adapter that translates MCP protocol calls into orchestrator-domain operations. New transports should mirror this seam.

## Decision

### Structure: Hexagonal-shaped folders

MSP source tree adopts the **Smart Proxy Pattern**:

```
packages/msp/src/
├── interfaces/              ← inbound adapters (ports)
│   ├── mcp/                 ← existing MCP server moves here
│   ├── slack/               ← (future) Slack webhook + Events API
│   ├── discord/             ← (future) Discord bot + interactions
│   ├── rest/                ← (future) HTTP REST API
│   └── cli/                 ← (future) interactive CLI shell
├── orchestrator/            ← application core (domain)
│   ├── router.ts            ← which tool/skill should handle this request
│   ├── correlation.ts       ← parallel fan-out + merge across clients
│   └── loop/                ← recall → think → act → retain → reflect
├── clients/                 ← outbound adapters (anti-corruption layers)
│   ├── gks-client.ts        ← wraps @freshair129/gks
│   ├── gitnexus-client.ts   ← (optional) code-intel peer
│   └── llm-client.ts        ← wraps Anthropic/OpenAI/Ollama
├── domain/                  ← types + invariants (Namespace, Session, Candidate, ...)
├── codegen/                 ← (existing) micro-task runner — stays as feature module
├── validator/               ← (existing) atom validator — stays as feature module
└── identity/                ← (existing) identity layer — stays as feature module
```

**Rule of dependency:**
- `interfaces/` may import from `orchestrator/`, `domain/`, `clients/`
- `orchestrator/` may import from `domain/`, `clients/` — **never** from `interfaces/`
- `clients/` may import from `domain/` only
- `domain/` imports nothing from these layers

### Naming choice: "interfaces" not "adapters"

JavaScript ecosystem reserves `adapters/` for many concepts (test adapters, db adapters). `interfaces/` is the Hexagonal "inbound port" terminology that maps clearly to what these files do: receive external requests and translate them into domain-language calls.

### Migration shape (deferred to BLUEPRINT)

This ADR fixes the **shape**. Concrete file moves (e.g. `src/mcp/` → `src/interfaces/mcp/`) and the order of refactor steps belong in a follow-up `[[BLUEPRINT--MSP-INTERFACE-LAYER]]` when ready to execute. Until that blueprint lands, MSP's current flat layout stays — this ADR is `status: proposed`.

### Anti-corruption boundary at `clients/`

GKS types (e.g. `RetrievalHit`, `Namespace` shape) must not leak into `orchestrator/` directly. `clients/gks-client.ts` performs DTO mapping:

```
GKS RetrievalHit  ──[gks-client.ts]──►  MSP RecallEvidence
                                         (orchestrator-domain type)
```

Reason: when GKS bumps its API in a minor release, only `gks-client.ts` needs updating — orchestrator code is insulated. This is the anti-corruption layer from DDD.

### Cross-cutting concerns at the interface seam

These responsibilities live in `interfaces/`, not orchestrator:

- **Authentication** — Slack signing, Discord interactions verification, REST bearer tokens
- **Idempotency / dedup** — Slack retries the same webhook on timeout; dedup via `X-Slack-Retry-Num` before invoking orchestrator
- **Rate limiting** — per-user / per-tenant throttling at the edge
- **Namespace resolution** — extract `{ tenant_id, user_id, agent_id, session_id }` from request headers/payload and stamp it onto a `MspContext` object before passing to orchestrator
- **Observability seam** — generate / propagate `traceId` (OpenTelemetry) at the boundary; orchestrator + clients inherit it

## Consequences

### Positive

- **Transport-portability** — adding Discord doesn't touch `orchestrator/`; rewrite is `interfaces/discord.ts` only
- **Testability** — orchestrator unit tests don't need to mock HTTP; they call domain functions directly
- **Extraction-ready** — when traffic justifies splitting Gateway to its own service, extract `interfaces/<x>` + thin HTTP client; orchestrator becomes a Service over RPC. Architecture decision doesn't change, only deployment.
- **Clear ownership** — interface owners (Slack expert, Discord expert) edit only `interfaces/<their>/`. Orchestrator owners (domain experts) edit only `orchestrator/`.
- **Matches existing MCP pattern** — `src/mcp/` is already a thin adapter. Other transports just follow the same recipe.

### Negative

- **Initial refactor cost** — moving `src/mcp/` → `src/interfaces/mcp/` touches imports across the project (~30-50 file edits depending on coupling). Quantify in the follow-up BLUEPRINT.
- **More boilerplate per request** — interface translates request → domain DTO → orchestrator call → domain DTO → interface translates response. For trivial passthroughs this feels indirect. Mitigation: keep the translation functions short (≤ 20 lines) and colocate with the interface.
- **Anti-corruption requires discipline** — easy to leak GKS types into orchestrator by accident. Mitigation: ESLint `no-restricted-imports` rule preventing `orchestrator/**` from importing `@freshair129/gks` directly (must go via `clients/gks-client.ts`).

### Neutral

- **Domain folder is new** — `src/domain/` doesn't exist today. It will accumulate `Namespace`, `Session`, `Candidate`, `RecallEvidence` types over time. Initially small.
- **Feature modules stay flat** — `codegen/`, `validator/`, `identity/` continue as siblings under `src/`. They're feature modules that may use `clients/` but aren't part of the request-response flow per se. Could be revisited if they grow.

## Alternatives considered

### A. Keep flat layout, add transports as siblings (`src/slack.ts`, `src/discord.ts`)

**Rejected because**: doesn't enforce the dependency rule — nothing stops `slack.ts` from being imported into orchestrator code. Becomes the same "MSP = Gateway" tight-coupling problem the user identified.

### B. Split Gateway into a separate package (`packages/msp-gateway/`)

**Rejected because**: premature. Two-package monorepo (`msp` + `gks`) is justified; three-package (`msp` + `msp-gateway` + `gks`) adds workspace ceremony for zero current benefit. Smart Proxy pattern inside MSP gives the same logical separation. Revisit when scaling pressure justifies the split (per §3 of the original analysis).

### C. Use a framework that enforces ports & adapters (NestJS, hexagonal-ts)

**Rejected because**: framework lock-in. Hexagonal is a pattern, not a framework. The discipline is encoded in folder structure + lint rules; we don't need a framework to enforce it. Adding a framework would be a separate, much larger ADR.

### D. Combined Layer (option 1 in the original analysis — MSP literally is the Gateway)

**Rejected because**: explicitly fails the dependency rule. Orchestrator ends up knowing about Slack signing, Discord interaction tokens, etc. Migration cost when transport changes = effective rewrite.

## What this ADR does NOT change

- **MSP↔GKS boundary** (`[[ADR--MONOREPO-STRUCTURE]]`, `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`) — GKS stays as outbound dependency; this ADR just formalizes where the wrapper lives (`clients/gks-client.ts`).
- **Public API surface** — MCP tool names, CLI commands, atom schemas — all unchanged. This is an internal refactor.
- **Candidates workflow** — `msp_candidate` MCP tool → `.brain/.../candidates/` → human PR review unchanged (per `[[ADR--AGENT-WRITE-BOUNDARIES]]`). The tool simply moves from `src/mcp/tools/candidate.ts` to `src/interfaces/mcp/tools/candidate.ts`.

## Status note

**Proposed, not implemented.** Implementation requires:
1. A `[[BLUEPRINT--MSP-INTERFACE-LAYER]]` specifying exact file moves + the order of refactor steps
2. ESLint config update (`no-restricted-imports` rule) to enforce the dependency direction
3. CI guard that fails if `orchestrator/**` imports from `interfaces/**` or `@freshair129/gks` directly
4. Migration PR (atomic — should be one PR per Smart Proxy pattern goal)

This ADR is filed as `proposed` to capture the decision; flip to `accepted` once the BLUEPRINT lands and refactor begins.

## Source

- Original analysis: discussion comparing OpenClaw / Hermes / GKS+MSP roles (2026-05-12)
- Hexagonal Architecture — Alistair Cockburn (2005), Domain-Driven Design — Eric Evans (2003)
- Prior art: NestJS module pattern, Spring's `@Controller` / `@Service` separation, Go's `cmd/` + `internal/` layout
- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` — establishes that MSP is the orchestrator above GKS
- `[[CONCEPT--AGENT-AGNOSTIC]]` — establishes that MSP must not assume any specific cognitive-layer client
- `[[ADR--MSP-MCP-SERVER]]` — establishes the first concrete interface (MCP server)
- `[[ADR--MONOREPO-STRUCTURE]]` — establishes the package boundary between MSP and GKS
