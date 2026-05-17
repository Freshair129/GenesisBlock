#!/usr/bin/env node
import { parseArgs } from 'node:util'
import { lstat, readdir, readFile, writeFile } from 'node:fs/promises'
import { resolve, join } from 'node:path'
import { parse as parseYaml, stringify as stringifyYaml } from 'yaml'

import { runClassifiers } from './engine.js'
import { PathClassifier } from './path.js'
import { ContentClassifier } from './content.js'
import { CodingClassifier } from './coding.js'
import { TaskClassifier } from './task.js'
import type { ClassifiableResource } from './types.js'

const HELP = `msp-tag — Automatic attribute tagging for GKS atoms

Usage:
  msp-tag [options] <files/dirs...>

Options:
  --root <dir>       Project root (default: .)
  --dry-run          Show changes without writing
  --verbose          Show provenance metadata
  --help             Show this message
`

async function main(): Promise<number> {
  const { values, positionals } = parseArgs({
    options: {
      root: { type: 'string' },
      'dry-run': { type: 'boolean' },
      verbose: { type: 'boolean' },
      help: { type: 'boolean' },
    },
    allowPositionals: true,
  })

  if (values.help || positionals.length === 0) {
    process.stdout.write(HELP)
    return 0
  }

  const root = resolve(values.root ?? '.')
  const classifiers = [
    new PathClassifier(),
    new ContentClassifier(),
    new CodingClassifier(),
    new TaskClassifier(),
  ]
  
  let updatedCount = 0
  let errorCount = 0

  async function processFile(relPath: string) {
    const absPath = resolve(root, relPath)
    try {
      const raw = await readFile(absPath, 'utf8')
      const fmMatch = raw.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)
      if (!fmMatch) return

      const fmText = fmMatch[1]
      const body = raw.slice(fmMatch[0].length)
      const fm = parseYaml(fmText)
      
      const resource: ClassifiableResource = {
        id: fm.id,
        path: relPath,
        body,
        // Pass the FULL frontmatter as attributes to classifiers.
        // The engine and classifiers can then pick what they need.
        attributes: { ...fm }
      }

      const result = await runClassifiers(resource, classifiers)
      
      // Check if anything changed
      const oldAttrText = JSON.stringify(fm.attributes || {})
      const newAttrText = JSON.stringify(result.attributes)

      if (oldAttrText !== newAttrText) {
        if (values.verbose) {
          process.stdout.write(`Tagging ${relPath}:\n`)
          for (const [k, v] of Object.entries(result.attributes)) {
            const prov = result.provenance[k]
            process.stdout.write(`  - ${k}: ${v} (${prov?.classifier_id})\n`)
          }
        } else if (!values['dry-run']) {
          process.stdout.write(`✓ ${relPath}\n`)
        } else {
          process.stdout.write(`? ${relPath} (would update)\n`)
        }

        if (!values['dry-run']) {
          fm.attributes = result.attributes
          const newFmText = stringifyYaml(fm).trim()
          await writeFile(absPath, `---\n${newFmText}\n---\n${body}`, 'utf8')
        }
        updatedCount++
      }
    } catch (err) {
      process.stderr.write(`✗ ${relPath}: ${(err as Error).message}\n`)
      errorCount++
    }
  }

  async function walk(dir: string) {
    const entries = await readdir(resolve(root, dir))
    for (const e of entries) {
      if (e === '.git' || e === 'node_modules' || e === '.obsidian' || e === '00_index') continue
      
      const relPath = join(dir, e)
      const stat = await lstat(resolve(root, relPath))
      
      if (stat.isDirectory()) {
        await walk(relPath)
      } else if (e.endsWith('.md')) {
        await processFile(relPath)
      }
    }
  }

  for (const pos of positionals) {
    const stat = await lstat(resolve(root, pos))
    if (stat.isDirectory()) {
      await walk(pos)
    } else {
      await processFile(pos)
    }
  }

  process.stdout.write(`\nSummary: ${updatedCount} atoms updated${values['dry-run'] ? ' (dry-run)' : ''}, ${errorCount} errors.\n`)
  return errorCount > 0 ? 1 : 0
}

main().then((code) => process.exit(code))
