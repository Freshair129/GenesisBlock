import { lock } from 'proper-lockfile';
import { appendFile } from 'node:fs/promises';
const DEFAULT_LOCK_OPTS = {
    stale: 10000,
    retries: 5,
    minTimeout: 100,
};
/**
 * Acquires a lock for the specified file and returns a release function.
 */
export async function lockSession(filePath, opts = DEFAULT_LOCK_OPTS) {
    // proper-lockfile requires the file to exist
    try {
        await appendFile(filePath, '', 'utf8');
    }
    catch (err) {
        // Ignore errors here
    }
    let releaseFunc;
    try {
        const lockOpts = {
            stale: opts.stale,
            retries: {
                retries: opts.retries,
                minTimeout: opts.minTimeout,
            },
        };
        if (opts.onStale) {
            lockOpts.onStale = opts.onStale;
        }
        else {
            lockOpts.onStale = (err) => {
                console.warn(`[lock] Stale lock detected for ${filePath}: ${err.message}`);
            };
        }
        releaseFunc = await lock(filePath, lockOpts);
    }
    catch (err) {
        throw new Error(`Failed to acquire session lock for ${filePath}: ${err.message}`);
    }
    return async () => {
        try {
            await releaseFunc();
        }
        catch (err) {
            console.error(`[lock] Failed to release lock for ${filePath}: ${err.message}`);
        }
    };
}
