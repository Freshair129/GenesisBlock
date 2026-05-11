#!/usr/bin/env node
/**
 * Set the executable bit on compiled bin entries after `tsc` outputs them.
 * tsc preserves the shebang line but strips file mode flags.
 */
import { chmodSync, statSync } from 'node:fs'

const targets = [
  'dist/validator/cli.js',
  'dist/memory/backlinks/cli.js',
  'dist/codegen/cli.js',
  'dist/codegen/master/cli.js',
  'dist/mcp/bin.js',
  'dist/symbols/cli.js',
]

for (const path of targets) {
  try {
    const s = statSync(path)
    chmodSync(path, s.mode | 0o111)
    console.log(`chmod +x ${path}`)
  } catch (err) {
    console.warn(`skip ${path}: ${err && err.message ? err.message : String(err)}`)
  }
}
