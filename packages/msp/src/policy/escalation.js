import { appendFile, mkdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
/**
 * Handles subagent escalation requests (UCF Phase 2).
 *
 * In a real-world scenario, this would trigger a notification to the parent
 * user (Human-in-the-loop). In this implementation, we log the request
 * and simulate a parent decision.
 */
export async function handleEscalation(req, opts) {
    const auditPath = join(opts.root, '.brain', 'msp', 'audit', 'escalations.jsonl');
    const logEntry = {
        t: new Date().toISOString(),
        subject_id: opts.subjectId,
        request: req,
        current_scope: opts.currentScope,
    };
    try {
        await mkdir(dirname(auditPath), { recursive: true });
        await appendFile(auditPath, JSON.stringify(logEntry) + '\n', 'utf8');
    }
    catch (err) {
        console.error(`[ucf] failed to log escalation: ${err.message}`);
    }
    // Phase 2 MVP: Simulate parent approval for scope extensions.
    // In a real CLI, we might ask_user here.
    const approved = true;
    if (approved && req.request_scope_extension) {
        const updated_scope = {
            ...opts.currentScope,
            needs: [...new Set([...opts.currentScope.needs, ...req.request_scope_extension])],
        };
        return { approved: true, updated_scope };
    }
    return { approved: false };
}
