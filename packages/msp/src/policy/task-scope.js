import { z } from 'zod';
/**
 * Task-level scope constraints.
 * See CONCEPT--SUBAGENT-CONTEXT-SCOPING.
 */
export const SubagentScopeSchema = z.object({
    /** Domains the subagent MUST have access to. */
    needs: z.array(z.string()).default([]),
    /** Domains that are nice to have but not critical. */
    nice_to_have: z.array(z.string()).default([]),
    /** Domains the subagent MUST NOT have access to. */
    excludes: z.array(z.string()).default([]),
});
export const SubagentBudgetSchema = z.object({
    /** Maximum tokens for context window. */
    tokens: z.number().int().positive().optional(),
    /** Whether the subagent is allowed to call expand(). */
    allow_expand: z.boolean().default(true),
    /** Maximum number of expand() calls. */
    expand_limit: z.number().int().nonnegative().optional(),
});
export function parseScope(raw) {
    return SubagentScopeSchema.parse(raw ?? {});
}
export function parseBudget(raw) {
    return SubagentBudgetSchema.parse(raw ?? {});
}
