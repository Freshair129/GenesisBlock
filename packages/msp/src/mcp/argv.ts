/**
 * Parse `--root=<path>` (or `--root <path>`) from argv.
 *
 * Returns the value if found, otherwise undefined — lets the server apply its
 * own MSP_ROOT / cwd fallback. Stops at the first match (so a later occurrence
 * of `--root` doesn't override an earlier one — convention matches CLI norms).
 *
 * Empty values (`--root=` with nothing after) are treated as missing.
 */
export function parseRootFromArgv(argv: readonly string[]): string | undefined {
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg.startsWith('--root=')) {
      const v = arg.slice('--root='.length)
      return v.length > 0 ? v : undefined
    }
    if (arg === '--root' && i + 1 < argv.length) {
      const v = argv[i + 1]
      return v.length > 0 ? v : undefined
    }
  }
  return undefined
}
