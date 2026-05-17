---
id: BLUEPRINT--MSP-MCP-TOOL-EXPANSION
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — MSP MCP tool expansion implementation plan
tags: &a1
  - msp
  - mcp
  - tools
  - blueprint
  - implementation
  - m7f
crosslinks: &a2
  implements:
    - FEAT--MSP-MCP-TOOL-EXPANSION
  references:
    - CONCEPT--MSP-MCP-TOOL-EXPANSION
    - FEAT--MSP-MCP-SERVER
linked_symbols: &a3
  - file: packages/msp/src/mcp/server.ts
  - file: packages/msp/src/mcp/tools/recall.ts
  - file: packages/msp/src/mcp/tools/remember.ts
  - file: packages/msp/src/mcp/tools/compress.ts
  - file: packages/msp/src/mcp/tools/identity-get.ts
  - file: packages/msp/src/mcp/tools/identity-set.ts
  - file: packages/msp/test/mcp/tools/recall.test.ts
  - file: packages/msp/test/mcp/tools/remember.test.ts
  - file: packages/msp/test/mcp/tools/compress.test.ts
  - file: packages/msp/test/mcp/tools/identity-get.test.ts
  - file: packages/msp/test/mcp/tools/identity-set.test.ts
created_at: 2026-05-05T16:15:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--MSP-MCP-TOOL-EXPANSION
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — MSP MCP tool expansion implementation plan
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T16:15:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--MSP-MCP-TOOL-EXPANSION
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — MSP MCP tool expansion implementation plan
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T16:15:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      domain: blueprint
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — MSP MCP tool expansion

```yaml
metadata:
  title: "MSP MCP tools — passport-side (recall/remember/compress/identity)"
  parent_feat: FEAT--MSP-MCP-TOOL-EXPANSION

architectural_pattern: |
  Five new tool handlers under src/mcp/tools/ following the existing M6
  pattern (closure-of-root → async args handler). Update src/mcp/server.ts
  to register them alongside the existing 6 tools.

  Each tool exports:
    export function handler(deps: { root: string }): (args: ToolArgs) => Promise<ToolResult>

  Where deps captures root + any other shared state (e.g. createSlmClient
  factory). The server wires handler() at startup.

  No new internal abstraction — each tool is ~50–100 lines including
  argument validation.

data_logic: |
  src/mcp/tools/recall.ts
    handler({ root }) → async ({ query, top_k, namespace, weights, timeout_ms }) => {
      const obsidian = await tryCreateObsidianClient({ root })  // best-effort
      const result = await recall({ query, root, namespace, obsidian,
                                    topK: top_k, timeoutMs: timeout_ms, weights })
      return { ok: true, ...result }
    }

  src/mcp/tools/remember.ts
    handler({ root }) → async ({ session_id, namespace, provider }) => {
      const llm = createSlmClient({ provider: provider ?? 'auto' })
      const episodes = await consolidate({ sessionId: session_id, root, namespace, llm })
      // Persist via existing EpisodicWriter
      const writer = new EpisodicWriter({ root, namespace })
      let persisted = 0
      for (const ep of episodes) {
        await writer.append(ep)
        persisted += 1
      }
      return { ok: true, episodes_emitted: episodes, episodes_persisted: persisted, ... }
    }

  src/mcp/tools/compress.ts
    handler({ root }) → async ({ episodes, budget_tokens, provider, preserve_order }) => {
      const llm = createSlmClient({ provider: provider ?? 'auto' })
      const result = await compress({ episodes, budgetTokens: budget_tokens, llm,
                                       preserveOrder: preserve_order })
      return { ok: true, ...result }
    }

  src/mcp/tools/identity-get.ts
    handler({ root }) → async ({ namespace, prune }) => {
      if (prune) await prunePreferences({ root, namespace })
      const identity = await getIdentity({ root, namespace })
      return { ok: true, identity }
    }

  src/mcp/tools/identity-set.ts
    handler({ root }) → async (args) => {
      switch (args.kind) {
        case 'profile':
          await setProfile({ root, namespace: args.namespace }, args.partial)
          break
        case 'voice':
          await setVoice({ root, namespace: args.namespace }, args.voice)
          break
        case 'preference':
          await setPreference({ root, namespace: args.namespace }, args.key, args.value, {
            expiresAt: args.expires_at,
            expiresInMs: args.expires_in_ms,
          })
          break
        default:
          throw new Error(`unknown kind: ${(args as any).kind}`)
      }
      const identity = await getIdentity({ root, namespace: args.namespace })
      return { ok: true, identity }
    }

  src/mcp/server.ts (update)
    register the 5 new tools in the existing tool array; tools/list now returns 11.

geography:
  - "packages/msp/src/mcp/tools/recall.ts"
  - "packages/msp/src/mcp/tools/remember.ts"
  - "packages/msp/src/mcp/tools/compress.ts"
  - "packages/msp/src/mcp/tools/identity-get.ts"
  - "packages/msp/src/mcp/tools/identity-set.ts"
  - "packages/msp/src/mcp/server.ts"                          # ← MODIFIED to register new tools
  - "packages/msp/test/mcp/tools/recall.test.ts"              # ~6 tests
  - "packages/msp/test/mcp/tools/remember.test.ts"            # ~5 tests (uses mock LLM + temp session.jsonl)
  - "packages/msp/test/mcp/tools/compress.test.ts"            # ~6 tests
  - "packages/msp/test/mcp/tools/identity-get.test.ts"        # ~5 tests
  - "packages/msp/test/mcp/tools/identity-set.test.ts"        # ~10 tests (3 kinds × variations)
  - "packages/msp/test/mcp/server.test.ts"                    # ← MODIFIED to assert 11 tools registered

verification_plan:
  - vitest recall: 6 tests — passes through; obsidian client absent → still works; weights override; timeout
  - vitest remember: 5 tests — episodes emitted, persisted via writer, llm_calls counted, mock session, missing session → empty
  - vitest compress: 6 tests — passes through; budget enforced; tier_counts correct; provider mock
  - vitest identity-get: 5 tests — empty default identity, with profile, with prune flag (TTL), with namespace, schemaVersion guard
  - vitest identity-set: 10 tests — 3 profile cases (set, partial, set-once createdAt), 2 voice cases, 4 preference cases (no TTL, expires_at, expires_in_ms, override), 1 unknown-kind error
  - vitest server: existing 4 tests + 1 assertion update — tools/list returns 11 tools

  Test count: ~430 → ~470 (+40)

implementation_order:
  T1 RECALL_TOOL    src/mcp/tools/recall.ts + 6 tests
  T2 REMEMBER_TOOL  src/mcp/tools/remember.ts + 5 tests
  T3 COMPRESS_TOOL  src/mcp/tools/compress.ts + 6 tests
  T4 IDENTITY_GET   src/mcp/tools/identity-get.ts + 5 tests
  T5 IDENTITY_SET   src/mcp/tools/identity-set.ts + 10 tests
  T6 SERVER_REG     update src/mcp/server.ts + update test/mcp/server.test.ts assertion
  T7 AUDIT          AUDIT--MSP-MCP-TOOL-EXPANSION
```

## Implementation notes

- **Run `npm ci`** in worktree per CLAUDE.md.
- **No new persistence layers** — each tool wraps existing TS APIs.
- **Don't change tool input/output for existing M6 tools** — additive only.
- **`tryCreateObsidianClient` helper** — wrap `createObsidianClient` in a try/catch returning `undefined` when env not configured. Inline at the top of `recall.ts` (small enough; could be extracted later).
- **`EpisodicWriter` instantiation** — existing class from `src/memory/episodic/writer.ts`; create a new instance per call (writers are cheap; no shared state).
- **Test pattern** — match `test/mcp/tools/validate.test.ts` (real-repo handler call returning JSON) and `test/mcp/tools/propose.test.ts` (mocking session writes).

## Implementer: do NOT do

- Modify M6 tool handlers (additive only)
- Add streaming output (out of scope)
- Add tool-level auth (M9)
- Cache results across calls
- Re-implement consolidate / recall / compress / identity inside the tool — wrap, don't reimplement

## Connections
- [[FEAT--MSP-MCP-TOOL-EXPANSION]]
- [[CONCEPT--MSP-MCP-TOOL-EXPANSION]]
- [[FEAT--MSP-MCP-SERVER]]

