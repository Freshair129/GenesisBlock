import { readFile } from 'node:fs/promises'

import { ValidatorIOError, type AtomicIndexEntry } from './types.js'

export async function loadAtomicIndex(
  path: string,
): Promise<Map<string, AtomicIndexEntry>> {
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code
    if (code === 'ENOENT') {
      throw new ValidatorIOError(
        `atomic index not found at ${path} — run \`npm run msp:index\` first`,
        err,
      )
    }
    throw new ValidatorIOError(`cannot read atomic index at ${path}`, err)
  }

  const map = new Map<string, AtomicIndexEntry>()
  const lines = raw.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!.trim()
    if (line === '') continue
    let entry: unknown
    try {
      entry = JSON.parse(line)
    } catch (err) {
      throw new ValidatorIOError(`atomic index: malformed JSON on line ${i + 1}`, err)
    }
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      throw new ValidatorIOError(`atomic index: line ${i + 1} is not an object`)
    }
    const e = entry as AtomicIndexEntry
    if (typeof e.id !== 'string') {
      throw new ValidatorIOError(`atomic index: line ${i + 1} missing id`)
    }
    map.set(e.id, e)
  }
  return map
}
