import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
export async function* walkMarkdown(dir) {
    let entries;
    try {
        entries = await readdir(dir, { withFileTypes: true });
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return;
        throw err;
    }
    for (const entry of entries) {
        const p = join(dir, entry.name);
        if (entry.isDirectory()) {
            if (entry.name === '00_index')
                continue;
            yield* walkMarkdown(p);
        }
        else if (entry.isFile() && entry.name.endsWith('.md')) {
            yield p;
        }
    }
}
