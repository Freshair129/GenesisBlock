import { mkdir, rename, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
/**
 * Write `content` to `path` atomically: write to `<path>.tmp`, then rename.
 * Rename is atomic on POSIX filesystems.
 */
export async function atomicWrite(path, content) {
    await mkdir(dirname(path), { recursive: true });
    const tmp = `${path}.tmp`;
    await writeFile(tmp, content, 'utf8');
    await rename(tmp, path);
}
