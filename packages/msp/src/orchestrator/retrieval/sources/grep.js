import { performance } from 'node:perf_hooks';
import { exec } from 'node:child_process';
import { promisify } from 'node:util';
import { DEFAULT_PER_SOURCE_TIMEOUTS, } from '../types.js';
const execAsync = promisify(exec);
/**
 * Retrieval source for substring search using git grep.
 */
export async function grepSource(opts) {
    const start = performance.now();
    const root = opts.root ?? process.cwd();
    const timeoutMs = opts.perSourceTimeouts?.grep ?? DEFAULT_PER_SOURCE_TIMEOUTS.grep;
    try {
        // Basic grep over gks/ folder
        // Note: this is a simple fallback for when semantic search or exact keyword match fails.
        const needle = opts.query.replace(/["\\]/g, ''); // Basic escaping
        const { stdout } = await execAsync(`git grep -i -l "${needle}" -- gks/`, {
            cwd: root,
            timeout: timeoutMs
        });
        const files = stdout.split('\n').filter(f => f.trim().length > 0);
        const hits = files.map((file, i) => {
            // Extract ID from filename (assuming filename matches ID)
            const idMatch = file.match(/([A-Z0-9_-]+)\.md$/);
            return {
                atomId: idMatch ? idMatch[1] : file,
                rank: i + 1,
                source: 'grep',
                memoryTier: 'atom' // Grep usually finds atoms
            };
        });
        return {
            source: 'grep',
            hits: hits.slice(0, 20),
            latencyMs: Math.round(performance.now() - start)
        };
    }
    catch (err) {
        // If grep finds nothing, it might exit with code 1
        return {
            source: 'grep',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            skipped: err.code === 1 ? 'no matches' : undefined,
            error: err.code !== 1 ? err.message : undefined
        };
    }
}
