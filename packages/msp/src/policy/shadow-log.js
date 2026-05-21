import { appendFile, mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
/**
 * Append an entry to the shadow policy log.
 */
export async function logShadowDecision(entry, logPath) {
    const fullEntry = {
        t: new Date().toISOString(),
        ...entry,
    };
    try {
        await mkdir(dirname(logPath), { recursive: true });
        await appendFile(logPath, JSON.stringify(fullEntry) + '\n', 'utf8');
    }
    catch (err) {
        console.error(`[policy] failed to write shadow log: ${err.message}`);
    }
}
