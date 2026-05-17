#!/usr/bin/env tsx
/**
 * migrate-aliases — Repo-wide migration to inject aliases frontmatter to GKS atoms.
 */
import { promises as fs } from 'node:fs'
import { join, resolve, relative, dirname } from 'node:path'
import { parse as yamlParse } from 'yaml'
import { fileURLToPath } from 'node:url'

import { buildAliases, lookupType } from '../../packages/msp/src/validator/utils/registry.ts'

const __filename = fileURLToPath(import.meta.url)
const REPO_ROOT = resolve(dirname(__filename), '..', '..')
const GKS_DIR = join(REPO_ROOT, 'gks')

const DRY_RUN = process.argv.includes('--dry-run')

async function fileExists(p: string): Promise<boolean> {
  try {
    await fs.access(p)
    return true
  } catch {
    return false
  }
}

async function listMarkdownFiles(dir: string): Promise<string[]> {
  const out: string[] = []
  async function walk(currentDir: string) {
    let entries
    try {
      entries = await fs.readdir(currentDir, { withFileTypes: true })
    } catch (err: any) {
      if (err.code === 'ENOENT') return
      throw err
    }
    for (const e of entries) {
      const full = join(currentDir, e.name)
      if (e.isDirectory()) {
        if (e.name === 'node_modules' || e.name === '.git' || e.name === 'dist' || e.name === '00_index') continue
        await walk(full)
      } else if (e.isFile() && e.name.endsWith('.md')) {
        out.push(full)
      }
    }
  }
  await walk(dir)
  return out
}

function processFrontmatter(fmText: string, aliases: string[], cluster: string, role: string): string {
  // Normalize line endings
  let lines = fmText.replace(/\r\n/g, '\n').split('\n')

  // Find if aliases section already exists and clean it up
  let startIdx = -1
  let endIdx = -1
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!.trim()
    if (line.startsWith('aliases:')) {
      startIdx = i
      // Find end of aliases block
      let j = i + 1
      while (j < lines.length) {
        const nextLine = lines[j]!
        const trimmedNext = nextLine.trim()
        if (trimmedNext === '') {
          j++
          continue
        }
        if (trimmedNext.startsWith('-') || nextLine.startsWith('  ') || nextLine.startsWith('\t')) {
          j++
        } else {
          break
        }
      }
      endIdx = j
      break
    }
  }

  if (startIdx !== -1 && endIdx !== -1) {
    lines.splice(startIdx, endIdx - startIdx)
  }

  // Remove existing cluster and role fields if present
  lines = lines.filter(line => !line.trim().startsWith('cluster:'))
  lines = lines.filter(line => !line.trim().startsWith('role:'))

  // Remove any trailing empty lines in lines
  while (lines.length > 0 && lines[lines.length - 1]!.trim() === '') {
    lines.pop()
  }

  lines.push('aliases:')
  for (const alias of aliases) {
    lines.push(`  - ${alias}`)
  }
  lines.push(`cluster: ${cluster}`)
  lines.push(`role: ${role}`)

  return lines.join('\n')
}

function healFrontmatter(fmText: string): string {
  let lines = fmText.replace(/\r\n/g, '\n').split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!.trim()
    if (line.startsWith('role:')) {
      const match = line.match(/^role:\s*(?:"([^"]+)"|(\S+(?:\s+\S+)*))\s+(\S+)\s*$/)
      if (match) {
        const roleVal = match[1] ?? match[2]!
        const clusterVal = match[3]!
        lines[i] = `role: ${JSON.stringify(roleVal)}`
        
        let foundCluster = false
        for (let j = 0; j < lines.length; j++) {
          if (lines[j]!.trim().startsWith('cluster:')) {
            lines[j] = `cluster: ${clusterVal}`
            foundCluster = true
            break
          }
        }
        if (!foundCluster) {
          lines.push(`cluster: ${clusterVal}`)
        }
      }
    }
  }
  return lines.join('\n')
}

async function migrateFile(filepath: string): Promise<boolean> {
  const content = await fs.readFile(filepath, 'utf8')
  const FRONTMATTER_DELIM = '---'

  if (!content.startsWith(FRONTMATTER_DELIM)) {
    console.warn(`[skip] ${relative(REPO_ROOT, filepath)}: missing leading ---`)
    return false
  }

  const end = content.indexOf(`\n${FRONTMATTER_DELIM}`, FRONTMATTER_DELIM.length)
  if (end === -1) {
    console.warn(`[skip] ${relative(REPO_ROOT, filepath)}: missing closing ---`)
    return false
  }

  let fmText = content.slice(FRONTMATTER_DELIM.length, end).trim()
  fmText = healFrontmatter(fmText)
  const body = content.slice(end + `\n${FRONTMATTER_DELIM}`.length)

  let fm: any
  try {
    fm = yamlParse(fmText)
  } catch (err: any) {
    console.error(`[error] failed to parse YAML in ${relative(REPO_ROOT, filepath)}: ${err.message}`)
    return false
  }

  if (!fm || typeof fm !== 'object') {
    console.warn(`[skip] ${relative(REPO_ROOT, filepath)}: frontmatter is not an object`)
    return false
  }

  const id = fm.id ?? fm.proposed_id
  if (!id || typeof id !== 'string') {
    console.warn(`[skip] ${relative(REPO_ROOT, filepath)}: missing id / proposed_id`)
    return false
  }

  const prefix = id.split('--')[0]!
  const typeDef = lookupType(prefix, REPO_ROOT)
  if (!typeDef) {
    console.warn(`[skip] ${relative(REPO_ROOT, filepath)}: no registry type definition for prefix ${prefix}`)
    return false
  }

  const targetAliases = buildAliases(id, fm.aliases, REPO_ROOT)
  const targetCluster = typeDef.cluster
  const targetRole = typeDef.role

  const aliasesCorrect = Array.isArray(fm.aliases) &&
                         fm.aliases.length === targetAliases.length &&
                         targetAliases.every((val, index) => fm.aliases[index] === val)
  const clusterCorrect = fm.cluster === targetCluster
  const roleCorrect = fm.role === targetRole

  if (aliasesCorrect && clusterCorrect && roleCorrect) {
    // Already correct!
    return false
  }

  const updatedFmText = processFrontmatter(fmText, targetAliases, targetCluster, targetRole)
  const finalDoc = `${FRONTMATTER_DELIM}\n${updatedFmText}\n${FRONTMATTER_DELIM}${body}`

  if (!DRY_RUN) {
    await fs.writeFile(filepath, finalDoc, 'utf8')
  }

  console.log(`${DRY_RUN ? '[dry-run]' : '[migrated]'} ${relative(REPO_ROOT, filepath)} -> cluster: ${targetCluster}, role: ${targetRole}, aliases: [${targetAliases.join(', ')}]`)
  return true
}

async function main() {
  console.log(`Starting GKS aliases migration... Mode: ${DRY_RUN ? 'DRY-RUN' : 'APPLY'}`)
  if (!(await fileExists(GKS_DIR))) {
    console.error(`GKS directory not found at ${GKS_DIR}`)
    process.exit(1)
  }

  const files = await listMarkdownFiles(GKS_DIR)
  console.log(`Found ${files.length} markdown files in ${relative(REPO_ROOT, GKS_DIR)}`)

  let migratedCount = 0
  for (const file of files) {
    try {
      const migrated = await migrateFile(file)
      if (migrated) migratedCount++
    } catch (err: any) {
      console.error(`Failed to migrate ${relative(REPO_ROOT, file)}: ${err.message}`)
    }
  }

  console.log(`\nMigration completed: ${migratedCount} files updated/migrated.`)
}

main().catch((err) => {
  console.error('Migration crashed:', err)
  process.exit(1)
})
