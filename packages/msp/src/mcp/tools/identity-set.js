import { resolve } from 'node:path';
import { z } from 'zod';
import { getIdentity, setPreference, setProfile, setVoice, writeIdentity, } from '../../identity/index.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_identity_set';
export const description = 'Mutate the passport identity. Discriminated by `kind`: `profile` partial-merges Profile fields (createdAt is set-once), `voice` full-replaces the Voice sub-field, `preference` sets a single preference key with optional TTL. `scope` controls write target — `global` writes to ~/.msp/identity.json (default for `profile`/`voice`), `project` writes to the workspace `identity.override.json` (sparse). Returns the merged identity post-write.';
const profileShape = z.object({
    name: z.string().optional(),
    role: z.string().optional(),
    tier: z.enum(['T1', 'T2', 'T3']).optional(),
    originStory: z.string().optional(),
    guardrails: z.array(z.string()).optional(),
    extensions: z.record(z.string(), z.unknown()).optional(),
});
const voiceShape = z.object({
    tone: z.array(z.string()),
    formality: z.enum(['casual', 'neutral', 'formal']),
    languagePreference: z.string(),
    responseCadence: z.enum(['terse', 'normal', 'verbose']),
});
export const inputSchema = {
    kind: z
        .enum(['profile', 'voice', 'preference'])
        .describe('Which sub-field to mutate.'),
    scope: z
        .enum(['global', 'project'])
        .optional()
        .describe("Write target. Defaults: profile → 'global', voice → 'global', preference → 'global'. Pass 'project' to write a sparse override at .brain/msp/projects/<ns>/identity.override.json instead."),
    partial: profileShape
        .optional()
        .describe('Partial profile fields (only for kind=profile).'),
    voice: voiceShape
        .optional()
        .describe('Full voice replacement (only for kind=voice).'),
    key: z
        .string()
        .optional()
        .describe('Preference key (only for kind=preference).'),
    value: z
        .unknown()
        .optional()
        .describe('Preference value, JSON-serialisable (only for kind=preference).'),
    expires_at: z
        .string()
        .optional()
        .describe('Preference TTL: absolute ISO 8601 expiry. Wins over expires_in_ms.'),
    expires_in_ms: z
        .number()
        .optional()
        .describe('Preference TTL: relative ms from now.'),
    namespace: z
        .string()
        .optional()
        .describe('Project namespace (default `evaAI`).'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
        const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
        const opts = { root, namespace: args.namespace, subject, context };
        // Sensible default per ADR: profile + voice → global, preferences → global.
        // Callers wanting a project-level override (per-project voice/role) pass
        // `scope: 'project'` explicitly.
        const scope = args.scope ?? 'global';
        try {
            switch (args.kind) {
                case 'profile': {
                    if (!args.partial) {
                        return errorResult('kind=profile requires `partial` field');
                    }
                    if (scope === 'global') {
                        await setProfile(opts, args.partial);
                    }
                    else {
                        // Project-scope partial profile: write a sparse override containing
                        // only the supplied profile fields. createdAt set-once is bypassed
                        // for overrides (the merged read picks up the global createdAt).
                        await writeIdentity({ ...opts, scope: 'project' }, { profile: args.partial });
                    }
                    break;
                }
                case 'voice': {
                    if (!args.voice) {
                        return errorResult('kind=voice requires `voice` field');
                    }
                    if (scope === 'global') {
                        await setVoice(opts, args.voice);
                    }
                    else {
                        await writeIdentity({ ...opts, scope: 'project' }, { voice: args.voice });
                    }
                    break;
                }
                case 'preference': {
                    if (typeof args.key !== 'string' || args.key.length === 0) {
                        return errorResult('kind=preference requires non-empty `key`');
                    }
                    if (!('value' in args)) {
                        return errorResult('kind=preference requires `value` field');
                    }
                    // setPreference always writes via writeIdentity (default scope=global).
                    // Project-scope prefs are not yet supported (would need a sparse merge
                    // path); reject with a clear error rather than silently routing global.
                    if (scope === 'project') {
                        return errorResult('kind=preference with scope=project is not yet supported; preferences write globally. Override per-project voice/profile via scope=project on those kinds instead.');
                    }
                    const ttl = args.expires_at !== undefined || args.expires_in_ms !== undefined
                        ? {
                            expiresAt: args.expires_at,
                            expiresInMs: args.expires_in_ms,
                        }
                        : undefined;
                    await setPreference(opts, args.key, args.value, ttl);
                    break;
                }
                default: {
                    // Defensive — Zod enum should reject this earlier.
                    return errorResult(`unknown kind: ${args.kind}`);
                }
            }
            const identity = await getIdentity(opts);
            return jsonResult({ ok: true, identity });
        }
        catch (err) {
            return errorResult(`identity_set failed: ${err.message}`);
        }
    };
}
