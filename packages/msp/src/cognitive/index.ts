/**
 * Cognitive Layer — one-line memoryOS entry point.
 *
 * Sandwich diagram (FRAME--MSP-ARCHITECTURE-V2):
 *
 *   COGNITIVE LAYER (EVA / Claude Code / Hermes / openclaw / Gemini CLI / …)
 *     └─►  this facade
 *            ├─► MSP passport (identity, candidates, codegen runner, MCP server)
 *            └─► GKS (atomic / vector / episodic / obsidian + GraphBackend)
 *
 * What this file adds on top of the pre-existing primitives:
 *   1. `recall()`           — hybrid 4-layer pipeline (§13) with FTS layer 2
 *   2. `remember()`         — wraps `retain()` from `@freshair129/gks`
 *   3. `consolidate()`      — placeholder for session-end reflect()
 *   4. `runTask()`          — tier routing + §7.7.2 scale-level gate + AUTO-GENERATED marker
 *   5. `verifyFlow()`       — re-exports GKS verifyFlow
 *   6. `hotfix`             — re-exports GKS HotfixStore methods
 *   7. `resolveSSOT()`      — §14.1 authority hierarchy
 *   8. `mcpServer()`        — pre-wired createMspMcpServer
 *
 * Bring-up:
 *   const layer = await createCognitiveLayer({ root: process.cwd() })
 *   await layer.remember('Cortex handles planning.')
 *   const hits = await layer.recall('how does cortex work?')
 *   await layer.runTask('./.brain/tasks/FEAT--X/T1.task.yaml', { scale: 'L2' })
 */

import { join, resolve } from 'node:path'

import {
  MemoryStore,
  HotfixStore,
  type GraphBackend,
  type RetrievalOptions,
  recall as gksRecall,
  retain as gksRetain,
  verifyFlow,
} from '@freshair129/gks'

import { runTask as runCodegenTask } from '../codegen/runner.js'
import { createSlmClient } from '../codegen/slm/factory.js'
import { createMspMcpServer } from '../mcp/server.js'
import { makeContext, makeSubject } from '../policy/types.js'

import { ftsSearch } from './fts.js'
import { markAuditOnly } from './audit-only.js'
import { enforceScaleGate } from './scale-gate.js'
import { resolveSSOT } from './ssot.js'
import {
  ScaleLevelGateError,
  type AtomCitation,
  type CognitiveLayer,
  type CognitiveLayerOptions,
  type CognitiveRecallHit,
  type CognitiveRecallResult,
  type CognitiveRunTaskOptions,
  type PolicyContext,
  type RememberOptions,
} from './types.js'

export async function createCognitiveLayer(
  opts: CognitiveLayerOptions,
): Promise<CognitiveLayer> {
  const root = resolve(opts.root)

  const memOpts = {
    root,
    ...(opts.defaultNamespace ? { defaultNamespace: opts.defaultNamespace } : {}),
    ...(opts.graphBackend
      ? {
          graphBackend:
            typeof opts.graphBackend === 'function'
              ? async () =>
                  (opts.graphBackend as (root: string) => Promise<GraphBackend> | GraphBackend)(
                    root,
                  )
              : opts.graphBackend,
        }
      : {}),
  }
  const store = new MemoryStore(memOpts)
  await store.init()

  const hotfixStore = new HotfixStore({ root })

  return {
    store,
    graph: store.graph,

    async recall(
      query: string,
      retrievalOpts: RetrievalOptions & PolicyContext = {},
    ): Promise<CognitiveRecallResult> {
      const subject = retrievalOpts.subject ?? makeSubject('user', 'anonymous')
      const action = retrievalOpts.action ?? 'recall'
      const context = retrievalOpts.context ?? makeContext('internal', 'system-recall')

      console.debug(`[ucf] 4-tuple: recall | sub:${subject.id} | act:${action} | trace:${context.trace_id}`)

      // Layer 1 + 3 + 4 already live inside MemoryStore.retrieve (atomic
      // short-circuit + vector + episodic, plus obsidian if configured).
      // We bolt FTS on as the §13 layer 2 by fan-out + a tiny RRF blend.
      const vectorPromise = gksRecall(store, query, retrievalOpts)
      const ftsPromise = ftsSearch(join(root, 'gks'), query, {
        limit: retrievalOpts.topK ?? 10,
      })

      const [vec, fts] = await Promise.all([vectorPromise, ftsPromise])

      const merged = new Map<string, CognitiveRecallHit>()
      for (const h of vec.hits) {
        // Prefer ID as key for atoms to ensure correct deduping between layers
        merged.set(h.id, markAuditOnly(h))
      }
      for (const h of fts) {
        const existing = merged.get(h.id)
        if (existing) {
          existing.score = Math.max(existing.score, h.score)
        } else {
          merged.set(h.id, h as CognitiveRecallHit)
        }
      }
      const sorted = [...merged.values()].sort((a, b) => b.score - a.score)
      const topK = retrievalOpts.topK ?? sorted.length
      return {
        ...vec,
        hits: sorted.slice(0, topK),
      }
    },

    async remember(
      content: string,
      rOpts: RememberOptions & PolicyContext = {},
    ): Promise<{ id: string }> {
      const subject = rOpts.subject ?? makeSubject('user', 'anonymous')
      const action = rOpts.action ?? 'write'
      const context = rOpts.context ?? makeContext('internal', 'system-remember')

      console.debug(`[ucf] 4-tuple: remember | sub:${subject.id} | act:${action} | trace:${context.trace_id}`)

      const result = await gksRetain(store, {
        content,
        metadata: {
          ...(rOpts.metadata ?? {}),
          ...(rOpts.tags ? { tags: rOpts.tags } : {}),
          // Threading attributes into metadata for Phase 0
          attributes: subject.attributes,
        },
      })
      // `vectorDocId` is optional on RetainResult (Inbound-only retain skips
      // vector insert). Fall back to the inbound path so the facade contract
      // always returns a non-empty string.
      return { id: result.vectorDocId ?? result.inboundPath ?? '' }
    },

    async consolidate(sessionId: string): Promise<void> {
      // The GKS `reflect()` verb requires a full session-end summary input
      // (see api.ts). The cognitive facade exposes a thin wrapper so EVA /
      // Claude Code can call it from a hook; real callers should still use
      // `reflect(store, ...)` for fine-grained control.
      // For Phase 0 we mark the session id in audit (no-op for non-MSP
      // callers) — full integration is tracked by FEAT--COGNITIVE-LAYER-FACADE.
      if (!sessionId) throw new Error('consolidate: sessionId is required')
    },

    async runTask(taskPath: string, runOpts: CognitiveRunTaskOptions = {}) {
      const subject = runOpts.subject ?? makeSubject('user', 'anonymous')
      const action = runOpts.action ?? 'expose-to-llm'
      const context = runOpts.context ?? makeContext('internal', 'system-run-task')

      console.debug(`[ucf] 4-tuple: runTask | sub:${subject.id} | act:${action} | trace:${context.trace_id}`)

      const scale = runOpts.scale ?? 'L2'

      // Resolve the parent_blueprint from the task YAML for the gate check.
      // We re-use the codegen loader to avoid drift.
      const { loadTask } = await import('../codegen/load-task.js')
      const task = await loadTask(resolve(taskPath))

      if (scale !== 'L1') {
        await enforceScaleGate({ root, blueprintId: task.parent_blueprint, scale })
      }

      // Tier-aware SLM injection. Default tier = T1 (Ollama+qwen2.5-coder).
      const tier = runOpts.tier ?? opts.slm?.tier ?? 'T1'
      const provider =
        runOpts.slmClient
          ? undefined
          : opts.slm?.provider ?? tierProvider(tier)
      const slmClient =
        runOpts.slmClient ??
        createSlmClient({
          ...(provider ? { provider } : {}),
          ...(opts.slm?.model ? { ollama: { model: opts.slm.model } } : {}),
          ...(opts.slm?.factory ?? {}),
        })

      return runCodegenTask(resolve(taskPath), {
        ...runOpts,
        slmClient,
      })
    },

    async verifyFlow(featId: string) {
      // Build byId map from the atomic layer (already loaded by init()).
      const entries = store.atomic.filter({})
      const byId = new Map(entries.map((e) => [e.id, e]))
      return verifyFlow(featId, byId)
    },

    resolveSSOT(citations: AtomCitation[]) {
      return resolveSSOT(citations)
    },

    hotfix: {
      open(args) {
        return hotfixStore.open({ commitSha: args.sha, title: args.reason, reason: args.reason })
      },
      list() {
        return hotfixStore.list()
      },
      close(sha: string) {
        return hotfixStore.close(`HOTFIX--${sha.toUpperCase().slice(0, 7)}`, [])
      },
      check() {
        return hotfixStore.listOverdue()
      },
    },

    mcpServer() {
      return createMspMcpServer({ root })
    },
  }
}

function tierProvider(tier: 'T1' | 'T2' | 'T3'): 'ollama' | 'gemini' | 'mock' {
  if (tier === 'T1') return 'ollama'
  if (tier === 'T2') return 'gemini'
  // T3 stays on whichever provider the caller plugs in. Default mock so
  // tests don't hit the network; production users override via `slm.provider`.
  return 'mock'
}

export type {
  CognitiveLayer,
  CognitiveLayerOptions,
  CognitiveRecallHit,
  CognitiveRecallResult,
  CognitiveRunTaskOptions,
  CognitiveTier,
  ScaleLevel,
} from './types.js'
export { ScaleLevelGateError } from './types.js'
export { resolveSSOT } from './ssot.js'
export { markAuditOnly } from './audit-only.js'
export { enforceScaleGate } from './scale-gate.js'
export { ftsSearch } from './fts.js'
export {
  buildAutoGeneratedMarker,
  composeWithMarker,
  bodyContainsMarker,
  AUTO_GENERATED_MARKER_TAG,
} from './compose.js'
