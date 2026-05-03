import { mkdir, rename, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

export async function atomicWrite(path: string, content: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true })
  const tmp = `${path}.tmp`
  await writeFile(tmp, content, 'utf8')
  await rename(tmp, path)
}
