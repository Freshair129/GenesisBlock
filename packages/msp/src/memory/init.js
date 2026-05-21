import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
/**
 * Initializes the required directory structure for the 8-8-8 memory system
 * within a specific namespace.
 */
export async function initMemoryStore(root, namespace) {
    const base = join(root, '.brain', 'msp', 'projects', namespace);
    const dirs = [
        join(base, 'memory', 'sessions'),
        join(base, 'memory', 'cores'),
        join(base, 'memory', 'spheres'),
        join(base, 'pending', 'raw'),
        join(base, 'pending', 'distill'),
        join(base, 'revisions'),
    ];
    for (const dir of dirs) {
        await mkdir(dir, { recursive: true });
    }
    console.log(`[memory] initialized store for namespace '${namespace}'`);
}
