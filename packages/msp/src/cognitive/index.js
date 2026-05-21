/**
 * Cognitive Layer — one-line memoryOS entry point.
 */
import { join, resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import { MemoryStore, HotfixStore, retain as gksRetain, verifyFlow, } from '@freshair129/gks';
import { runTask as runCodegenTask } from '../codegen/runner.js';
import { createSlmClient } from '../codegen/slm/factory.js';
import { createMspMcpServer } from '../mcp/server.js';
import { makeContext, makeResource, makeSubject } from '../policy/types.js';
import { loadPolicies } from '../policy/loader.js';
import { evaluatePolicy } from '../policy/pdp.js';
import { logShadowDecision } from '../policy/shadow-log.js';
import { handleEscalation } from '../policy/escalation.js';
import { enforcePolicy } from '../policy/pep.js';
import { recall as mspRecall } from '../orchestrator/retrieval/index.js';
import { markAuditOnly } from './audit-only.js';
import { enforceScaleGate } from './scale-gate.js';
import { resolveSSOT } from './ssot.js';
export async function createCognitiveLayer(opts) {
    const root = resolve(opts.root);
    const memOpts = {
        root,
        ...(opts.defaultNamespace ? { defaultNamespace: opts.defaultNamespace } : {}),
        ...(opts.graphBackend
            ? {
                graphBackend: typeof opts.graphBackend === 'function'
                    ? async () => opts.graphBackend(root)
                    : opts.graphBackend,
            }
            : {}),
    };
    const store = new MemoryStore(memOpts);
    await store.init();
    const hotfixStore = new HotfixStore({ root });
    const policiesDir = join(root, 'policies');
    const policySet = await loadPolicies(policiesDir);
    return {
        store,
        graph: store.graph,
        async recall(query, retrievalOpts = {}) {
            const subject = retrievalOpts.subject ?? makeSubject('user', 'anonymous');
            const action = retrievalOpts.action ?? 'recall';
            const context = retrievalOpts.context ?? makeContext('internal', 'system-recall');
            console.debug(`[ucf] 4-tuple: facade.recall | sub:${subject.id} | act:${action} | trace:${context.trace_id}`);
            const result = await mspRecall({
                query,
                root,
                namespace: opts.defaultNamespace?.tenant_id,
                topK: retrievalOpts.topK,
                subject,
                context,
                embedder: await store.embedder(),
                vectorBackend: await store.getVectorStore('atomic'),
            });
            const hits = result.hits.map((h) => {
                const hit = {
                    id: h.atomId,
                    atomId: h.atomId,
                    source: h.source === 'gks-vector' ? 'vector' : h.source,
                    score: h.score,
                    snippet: h.snippet ?? '',
                    metadata: {
                        ...h.attributes,
                        perSourceRanks: h.perSourceRanks,
                    },
                };
                return markAuditOnly(hit);
            });
            return {
                query,
                hits,
                strategy: 'multi',
                tookMs: result.timings.fusion,
                fallback_reasons: result.fallback_reasons,
            };
        },
        async remember(content, rOpts = {}) {
            const subject = rOpts.subject ?? makeSubject('user', 'anonymous');
            const action = rOpts.action ?? 'write';
            const context = rOpts.context ?? makeContext('internal', 'system-remember');
            console.debug(`[ucf] 4-tuple: remember | sub:${subject.id} | act:${action} | trace:${context.trace_id}`);
            const result = await gksRetain(store, {
                content,
                metadata: {
                    ...(rOpts.metadata ?? {}),
                    ...(rOpts.tags ? { tags: rOpts.tags } : {}),
                    attributes: subject.attributes,
                },
            });
            return { id: result.vectorDocId ?? result.inboundPath ?? '' };
        },
        async escalate(req) {
            return handleEscalation(req, {
                root,
                currentScope: { needs: [], nice_to_have: [], excludes: [] },
                subjectId: 'anonymous-subagent',
            });
        },
        async expand(req, pOpts) {
            const subject = pOpts?.subject ?? makeSubject('user', 'anonymous');
            const action = pOpts?.action ?? 'read';
            const context = pOpts?.context ?? makeContext('internal', 'system-expand');
            const targetTier = req.to ?? 'FULL';
            const atom = await store.lookup(req.id);
            if (!atom) {
                throw new Error(`expand: atom not found: ${req.id}`);
            }
            // Read full body first to allow content-based policy checks (UCF Phase 4)
            const absPath = resolve(root, 'gks', atom.path);
            const raw = await readFile(absPath, 'utf8');
            const body = raw.split('\n---').pop()?.trim() ?? '';
            const resource = makeResource('atom', req.id, {}, { ...(atom.attributes ?? {}), body });
            const { permitted, decision } = await enforcePolicy(resource, { root, subject, action, context });
            if (!permitted) {
                return {
                    id: req.id,
                    tier: 'MENTION',
                    denied_reason: decision.reasoning[0]?.description ?? 'Denied by policy',
                };
            }
            const hit = {
                id: req.id,
                body,
                tier: targetTier,
            };
            return hit;
        },
        async consolidate(sessionId) {
            if (!sessionId)
                throw new Error('consolidate: sessionId is required');
        },
        async runTask(taskPath, runOpts = {}) {
            const { loadTask } = await import('../codegen/load-task.js');
            const task = await loadTask(resolve(taskPath));
            const subject = runOpts.subject ?? makeSubject('subagent', task.id, { scope: task.scope ?? { needs: [], nice_to_have: [], excludes: [] } });
            const action = runOpts.action ?? 'expose-to-llm';
            const context = runOpts.context ?? makeContext('internal', `task-${task.id}-${Date.now()}`);
            console.debug(`[ucf] 4-tuple: runTask | sub:${subject.id} | act:${action} | trace:${context.trace_id}`);
            // Phase 1: Shadow PEP (Task Level)
            const resource = makeResource('context-slot', 'run-task-execution');
            const decision = evaluatePolicy(subject, resource, action, context, policySet);
            const logPath = join(root, '.brain', 'msp', 'audit', 'shadow-policy.jsonl');
            await logShadowDecision({
                trace_id: context.trace_id,
                subject,
                resource,
                action,
                context,
                decision,
                policy_version: policySet.version,
            }, logPath);
            if (decision.effect === 'deny') {
                console.warn(`[ucf] shadow-deny: runTask would have been denied for ${subject.id}`);
            }
            const scale = runOpts.scale ?? 'L2';
            if (scale !== 'L1') {
                await enforceScaleGate({ root, blueprintId: task.parent_blueprint, scale });
            }
            const tier = runOpts.tier ?? opts.slm?.tier ?? 'T1';
            const provider = runOpts.slmClient
                ? undefined
                : opts.slm?.provider ?? tierProvider(tier);
            const slmClient = runOpts.slmClient ??
                createSlmClient({
                    ...(provider ? { provider } : {}),
                    ...(opts.slm?.model ? { ollama: { model: opts.slm.model } } : {}),
                    ...(opts.slm?.factory ?? {}),
                });
            return runCodegenTask(resolve(taskPath), {
                ...runOpts,
                slmClient,
            });
        },
        async verifyFlow(featId) {
            const entries = store.atomic.filter({});
            const byId = new Map(entries.map((e) => [e.id, e]));
            return verifyFlow(featId, byId);
        },
        resolveSSOT(citations) {
            return resolveSSOT(citations);
        },
        hotfix: {
            open(args) {
                return hotfixStore.open({ commitSha: args.sha, title: args.reason, reason: args.reason });
            },
            close(sha) {
                return hotfixStore.close(`HOTFIX--${sha.toUpperCase().slice(0, 7)}`, []);
            },
            list() {
                return hotfixStore.list();
            },
            check() {
                return hotfixStore.listOverdue();
            },
        },
        mcpServer() {
            return createMspMcpServer({ root });
        },
    };
}
function tierProvider(tier) {
    if (tier === 'T1')
        return 'ollama';
    if (tier === 'T2')
        return 'gemini';
    return 'mock';
}
export { ScaleLevelGateError } from './types.js';
export { resolveSSOT } from './ssot.js';
export { markAuditOnly } from './audit-only.js';
export { enforceScaleGate } from './scale-gate.js';
export { ftsSearch } from './fts.js';
export { buildAutoGeneratedMarker, composeWithMarker, bodyContainsMarker, AUTO_GENERATED_MARKER_TAG, } from './compose.js';
