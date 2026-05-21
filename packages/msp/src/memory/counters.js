import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import lockfile from 'proper-lockfile';
const DEFAULT_COUNTERS = {
    session_seq: 0,
    core_seq: 0,
    sphere_seq: 0,
    total_sessions: 0,
    last_updated: new Date().toISOString(),
};
/**
 * Increments the memory counters for a specific tier and updates the state file.
 * Uses advisory locking to ensure atomic updates across processes.
 */
export async function updateMemoryCounters(root, namespace, incrementType) {
    const counterPath = join(root, '.brain', 'msp', 'projects', namespace, 'memory', 'counters.json');
    // Ensure directory exists
    await mkdir(dirname(counterPath), { recursive: true });
    // Initialize file if missing
    try {
        await readFile(counterPath, 'utf8');
    }
    catch (err) {
        if (err.code === 'ENOENT') {
            await writeFile(counterPath, JSON.stringify(DEFAULT_COUNTERS, null, 2), 'utf8');
        }
    }
    // Acquire lock
    const release = await lockfile.lock(counterPath, { stale: 5000, retries: 5 });
    try {
        const raw = await readFile(counterPath, 'utf8');
        const state = JSON.parse(raw);
        state.total_sessions++;
        state.last_updated = new Date().toISOString();
        if (incrementType === 'session') {
            state.session_seq++;
        }
        else if (incrementType === 'core') {
            state.core_seq++;
            state.session_seq = 0; // Reset session count after core synthesis
        }
        else if (incrementType === 'sphere') {
            state.sphere_seq++;
            // Note: core_seq reset logic depends on narratives_per_identity setting
        }
        await writeFile(counterPath, JSON.stringify(state, null, 2), 'utf8');
        return state;
    }
    finally {
        await release();
    }
}
/**
 * Reads the current state of memory counters without incrementing.
 */
export async function readMemoryCounters(root, namespace) {
    const counterPath = join(root, '.brain', 'msp', 'projects', namespace, 'memory', 'counters.json');
    try {
        const raw = await readFile(counterPath, 'utf8');
        return JSON.parse(raw);
    }
    catch (err) {
        return DEFAULT_COUNTERS;
    }
}
