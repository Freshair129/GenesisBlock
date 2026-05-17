---
id: AUDIT--COGNITIVE-LAYER-V0
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: direct_experience
title: AUDIT — Cognitive Layer V0 (Genesis Block + PgGraph + Gemini-as-subagent
  + createCognitiveLayer facade)
tags: &a1
  - msp
  - gks
  - cognitive-layer
  - genesis-block
  - gemini
  - audit
crosslinks: &a2
  references:
    - CONCEPT--COGNITIVE-LAYER-FACADE
    - CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER
    - ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER
    - ADR--GEMINI-AS-SLM-PROVIDER
    - ADR--MEMORY-STORE-GRAPH-WIRING
    - BLUEPRINT--GENESIS-GRAPH-TS-FIRST
    - FEAT--COGNITIVE-LAYER-FACADE
linked_symbols: &a3
  - file: packages/gks/src/memory/graph/genesis-graph.ts
  - file: packages/gks/src/memory/graph/cypher-v0.ts
  - file: packages/gks/src/memory/graph/genesis-graph-errors.ts
  - file: packages/gks/src/memory/index.ts
  - file: packages/msp/src/cognitive/index.ts
  - file: packages/msp/src/cognitive/types.ts
  - file: packages/msp/src/cognitive/fts.ts
  - file: packages/msp/src/cognitive/scale-gate.ts
  - file: packages/msp/src/cognitive/ssot.ts
  - file: packages/msp/src/cognitive/audit-only.ts
  - file: packages/msp/src/cognitive/compose.ts
  - file: packages/msp/src/codegen/slm/gemini.ts
  - file: packages/msp/src/codegen/runner.ts
  - file: packages/msp/examples/cognitive-layer-quickstart.ts
created_at: 2026-05-12T22:55:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--COGNITIVE-LAYER-V0
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: direct_experience
  title: AUDIT — Cognitive Layer V0 (Genesis Block + PgGraph + Gemini-as-subagent
    + createCognitiveLayer facade)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-12T22:55:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--COGNITIVE-LAYER-V0
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: direct_experience
    title: AUDIT — Cognitive Layer V0 (Genesis Block + PgGraph + Gemini-as-subagent
      + createCognitiveLayer facade)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-12T22:55:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — Cognitive Layer V0

## Scope

Stand up the cognitive-layer / memoryOS stack end-to-end so EVA, Claude Code, Hermes, openclaw, and Gemini CLI can adopt it with one import, satisfying the user request: "ระบบ Cognitive Layer memoryOS — MSP Knowledge Base — GKS GraphBackend — Genesis Block, Pggraph — complete and ready for use with EVA or Claude Code, to compete with openclaw / Hermes, with the ability to call Gemini CLI as a subagent for code writing."

## What shipped

### GKS — storage primitives
1. `packages/gks/src/memory/graph/genesis-graph.ts` — `class GenesisGraphBackend implements GraphBackend`. Event-replay JSONL store (`<dir>/genesis-block.jsonl` + `manifest.json` version byte). Mirrors `GraphStore` semantics + adds `cypher(query)`.
2. `packages/gks/src/memory/graph/cypher-v0.ts` — hand-written recursive-descent parser covering `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §"Cypher v0 scope". Throws `GenesisGraphUnsupportedCypher` for anything outside the subset.
3. `packages/gks/src/memory/graph/genesis-graph-errors.ts` — `GenesisGraphUnsupportedCypher` + `GenesisGraphSchemaMismatchError`.
4. `MemoryStoreOptions.graphBackend` — new opt-in knob; default = JSONL-backed `GraphStore` at `<brain>/graph/graph.jsonl`. `store.graph` is now part of the public surface after `init()`. ([[ADR--MEMORY-STORE-GRAPH-WIRING]])
5. `HotfixStore` + `HotfixStoreOptions` + `OpenHotfixArgs` now exported from `@freshair129/gks`.
6. `.gitignore` extended for `*.genesis-block.jsonl` and `.brain/**/graph/*.jsonl`.

### MSP — SLM tier
7. `packages/msp/src/codegen/runner.ts` — removed the contradictory `?? 'qwen'` provider fallback. Default microtask SLM is now Ollama + `qwen2.5-coder:7b`, matching `[[CONCEPT--CODEGEN-MICROTASK-RUNNER]]` and `FRAMEWORK_MASTER_SPEC` §17.3. ([[ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER]])
8. `packages/msp/src/codegen/cli.ts` — help text + provider-selection block.
9. `packages/msp/src/codegen/slm/gemini.ts` (NEW) — `createGeminiClient()` + shared `runGeminiCli()` wrapper around `gemini -p <prompt> -y`. Used by both the SLM factory and the existing escalator. ([[ADR--GEMINI-AS-SLM-PROVIDER]])
10. `packages/msp/src/codegen/slm/factory.ts` + `slm/types.ts` — `'gemini'` added to provider union with `GeminiOpts`.
11. `packages/msp/src/codegen/escalator/gemini.ts` — refactored to consume `runGeminiCli()`; no behaviour change.

### MSP — cognitive layer facade
12. `packages/msp/src/cognitive/index.ts` (NEW) — `createCognitiveLayer({ root, graphBackend, slm, defaultNamespace })` returns `{ recall, remember, consolidate, runTask, verifyFlow, hotfix, resolveSSOT, mcpServer, store, graph }`. ([[FEAT--COGNITIVE-LAYER-FACADE]])
13. `packages/msp/src/cognitive/types.ts` — `CognitiveTier`, `ScaleLevel`, `CognitiveLayerOptions`, `CognitiveRecallHit`, `ScaleLevelGateError`.
14. `packages/msp/src/cognitive/fts.ts` (NEW) — pure-Node FTS for §13 layer 2.
15. `packages/msp/src/cognitive/scale-gate.ts` (NEW) — §7.7.2 L1/L2/L3 enforcement (`enforceScaleGate`).
16. `packages/msp/src/cognitive/ssot.ts` (NEW) — §14.1 authority hierarchy resolver.
17. `packages/msp/src/cognitive/audit-only.ts` (NEW) — §7.5 Memory-for-Audit stamping.
18. `packages/msp/src/cognitive/compose.ts` + `marker-constants.ts` (NEW) — §9.6 AUTO-GENERATED marker helper.
19. `packages/msp/src/index.ts` — re-exports `createCognitiveLayer` + cognitive types.

### Quickstart + docs
20. `packages/msp/examples/cognitive-layer-quickstart.ts` — 80-line end-to-end demo with GenesisGraphBackend + cypher round-trip + SSOT resolution + MCP tool surface print.
21. `packages/msp/package.json` — `"cognitive:quickstart": "tsx examples/cognitive-layer-quickstart.ts"`.

### Atoms authored
- `[[CONCEPT--COGNITIVE-LAYER-FACADE]]`
- `[[CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER]]`
- `[[ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER]]`
- `[[ADR--GEMINI-AS-SLM-PROVIDER]]`
- `[[ADR--MEMORY-STORE-GRAPH-WIRING]]`
- `[[BLUEPRINT--GENESIS-GRAPH-TS-FIRST]]`
- `[[FEAT--COGNITIVE-LAYER-FACADE]]`
- `[[PROTO--AUTO-GENERATED-MARKER]]`
- `[[PROTO--SCALE-LEVEL-GATE]]`
- (this) `[[AUDIT--COGNITIVE-LAYER-V0]]`

## Verification

| Check | Result |
|---|---|
| `npm run typecheck` (gks) | 0 |
| `npm run typecheck` (msp) | 0 |
| `npm test --workspace=packages/gks` | 349 passed, 3 skipped (was 333; +16 from new Genesis Block / memory-store tests) |
| `npm test --workspace=packages/msp` | 710 passed, 1 pre-existing flake unrelated to this PR (`test/mcp/bin.test.ts > uses --root=<path>` — verified failing identically on `main`) |
| Cognitive tests (`test/cognitive/*` + `test/codegen/slm/gemini.test.ts` + `test/codegen/runner-default-slm.test.ts`) | 26 passed |
| `npm run msp:index` | 241 atoms indexed (was 231; +9 net + AUDIT-- + index regen) |
| `npm run msp:check-links` via `gks validate --links` | OK |
| Atom validator `--all` | 241 passed, 0 failed |
| Quickstart smoke (`tsx examples/cognitive-layer-quickstart.ts`) | end-to-end success (remember + recall + cypher v0 round-trip + 20 MCP tools listed + SSOT resolution) |

## FRAMEWORK_MASTER_SPEC alignment (9 §-references honoured)

| § | Mechanism in this PR |
|---|---|
| §5.1 / §5.4 | `recall` consults atomic exact-id short-circuit before fan-out |
| §7.5 | `audit-only.ts` stamps episodic hits |
| §7.7.2 | `scale-gate.ts` enforced in `runTask` |
| §8.1 / §17.3 | `runTask({ tier })` T1=Ollama+qwen2.5-coder, T2=Gemini, T3=caller-supplied |
| §9.6 | `compose.ts` AUTO-GENERATED marker; validated by `[[PROTO--AUTO-GENERATED-MARKER]]` |
| §13 | 4-layer hybrid pipeline — FTS added as layer 2 (`fts.ts`); atomic / vector / graph already shipped |
| §14.1 | `ssot.ts::resolveSSOT` |
| §6.4 | `cognitive.hotfix.{open,list,close,check}` re-exports |
| §17.1 | Path encoding flows through existing `projects/resolve.ts` |

## Deviations from [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]

The Rust crate at `packages/gks/native/genesis-block/` is **not** shipped in this PR (Phase 0 = TS-only). The TS-first staging is captured by `[[BLUEPRINT--GENESIS-GRAPH-TS-FIRST]]` and uses the same directory layout the Rust binary will eventually own, so the upgrade is invisible to consumers. Phases P3.1–P3.6 in the original BLUEPRINT remain as written.

## Outstanding (post-PR)

- Atom statuses are `draft` — promotion to `stable` happens after CI green on Node 20 + 22 (per `packages/msp/CLAUDE.md` branch convention).
- Rust crate ([[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]] P3.1–P3.6).
- Full §8.4 slot/layout grammar for the deterministic composer.
- Pre-existing `test/mcp/bin.test.ts > uses --root=<path>` flake (environment-related; reproduces on `main`).

## Bottom line

The user's five requested components are all complete and shipped:

| Requested | Where it lives now |
|---|---|
| **Cognitive Layer / memoryOS** | `createCognitiveLayer()` facade in MSP — `packages/msp/src/cognitive/` |
| **MSP Knowledge Base** | `gks/` atoms via `msp_candidate` + PR; the cognitive facade exposes the surface |
| **GKS GraphBackend** | `graphBackend` opt added to `MemoryStoreOptions`; `store.graph` on the public class |
| **Genesis Block** | `createGenesisGraphBackend()` exported from `@freshair129/gks` (Phase 0 TS, Rust later) |
| **PgGraph** | already shipped at `packages/gks/src/memory/graph/pg.ts` — now reachable through the facade's `graphBackend` knob |
| **Gemini CLI as subagent** | `MSP_SLM_PROVIDER=gemini` (first-class SLM) + existing escalator path |
