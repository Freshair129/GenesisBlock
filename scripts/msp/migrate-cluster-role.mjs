#!/usr/bin/env node
// One-shot migration: inject cluster: and role: into every GKS atom that lacks them.
import { readdirSync, readFileSync, writeFileSync, statSync } from 'node:fs'
import { join, resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parse as yamlParse } from 'yaml'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(__dirname, '..', '..')
const GKS_DIR = join(ROOT, 'gks')
const SCHEMA_PATH = join(ROOT, 'atom_schema.yaml')

const schema = yamlParse(readFileSync(SCHEMA_PATH, 'utf8'))

// Flatten taxonomy into prefix → {cluster, role}
const typeMap = {}
for (const [clusterName, cluster] of Object.entries(schema.taxonomy.clusters)) {
  for (const [typeId, config] of Object.entries(cluster.types)) {
    typeMap[typeId.toLowerCase()] = { cluster: clusterName, role: config.role }
  }
}

function walk(dir) {
  const out = []
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name)
    if (entry.isDirectory()) {
      if (['node_modules', '.git', 'dist', '00_index'].includes(entry.name)) continue
      out.push(...walk(full))
    } else if (entry.isFile() && entry.name.endsWith('.md')) {
      out.push(full)
    }
  }
  return out
}

const DRY_RUN = process.argv.includes('--dry-run')
let patched = 0, skipped = 0

for (const file of walk(GKS_DIR)) {
  const raw = readFileSync(file, 'utf8')
  if (!raw.startsWith('---')) { skipped++; continue }

  const end = raw.indexOf('\n---', 3)
  if (end === -1) { skipped++; continue }

  const fmRaw = raw.slice(3, end)
  const fm = yamlParse(fmRaw)

  // Already has both fields
  if (fm.cluster !== undefined && fm.role !== undefined) { skipped++; continue }

  const prefix = (fm.id ?? '').split('--')[0]?.toLowerCase()
  const typeDef = prefix ? typeMap[prefix] : null
  if (!typeDef) { skipped++; continue }

  // Need to inject missing fields after 'aliases:' block (or before 'tags:')
  let updated = raw.slice(0, end + 4) // include closing ---
  const body = raw.slice(end + 4)

  const inFm = raw.slice(0, end + 4)
  let newFm = inFm

  if (fm.cluster === undefined) {
    // Insert after aliases block or before tags
    const insertAfter = /^(aliases:(?:\r?\n  -[^\r\n]*)*)/m
    const clusterLine = `cluster: ${typeDef.cluster}`
    if (insertAfter.test(newFm)) {
      newFm = newFm.replace(insertAfter, `$1\n${clusterLine}`)
    } else {
      newFm = newFm.replace(/^(tags:)/m, `${clusterLine}\n$1`)
    }
  }

  if (fm.role === undefined) {
    const roleLine = `role: ${JSON.stringify(typeDef.role)}`
    // Replace the full cluster line (including its value) to insert role after it
    newFm = newFm.replace(/^(cluster:[^\r\n]*)/m, `$1\n${roleLine}`)
  }

  if (newFm === inFm) { skipped++; continue }

  if (!DRY_RUN) writeFileSync(file, newFm + body)
  patched++
}

console.log(`patched: ${patched}, skipped: ${skipped}${DRY_RUN ? ' (dry-run)' : ''}`)
