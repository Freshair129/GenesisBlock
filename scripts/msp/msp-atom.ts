#!/usr/bin/env tsx
/**
 * msp-atom — 3-mode CLI for registry-driven atom authoring.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { parse as yamlParse } from 'yaml'

import { buildAliases, lookupType } from '../../packages/msp/src/validator/utils/registry.ts'

interface Args {
  command?: string
  type?: string
  slug?: string
  title?: string
  root?: string
  force?: boolean
  'body-from'?: string
}

const args: Args = {}
const positional: string[] = []

for (const arg of process.argv.slice(2)) {
  const m = arg.match(/^--([a-z-]+)(?:=(.*))?$/)
  if (m) {
    const [, key, value] = m
    if (key === 'force') {
      args.force = true
    } else if (value !== undefined) {
      ;(args as Record<string, string>)[key!] = value
    }
  } else {
    positional.push(arg)
  }
}

args.command = positional[0]

function die(msg: string): never {
  console.error(`msp-atom: ${msg}`)
  process.exit(1)
}

if (!args.command || !['prompt', 'create', 'scaffold'].includes(args.command)) {
  die('usage: msp-atom <prompt|create|scaffold> --type=<type> ...')
}

if (!args.type) die('missing --type (e.g. --type=feat)')

// Load registry
const rootDir = resolve(args.root ?? process.cwd())
const registryPath = join(rootDir, 'atom_registry.yaml')
if (!existsSync(registryPath)) {
  die(`could not find atom_registry.yaml at ${registryPath}`)
}

let registry: any
try {
  registry = yamlParse(readFileSync(registryPath, 'utf8'))
} catch (e: any) {
  die(`failed to parse atom_registry.yaml: ${e.message}`)
}

const type = args.type.toLowerCase()

// Flatten taxonomy for quick lookup
const TYPE_CONFIG: Record<string, { phase: number; folder: string; tier: string; sections: string[]; db_id?: string }> = {}
for (const cluster of Object.values(registry.taxonomy.clusters) as any[]) {
  for (const [id, config] of Object.entries(cluster.types) as [string, any][]) {
    TYPE_CONFIG[id.toLowerCase()] = config
  }
}

const config = TYPE_CONFIG[type]
if (!config) {
  die(`unknown --type '${args.type}' — must be one of: ${Object.keys(TYPE_CONFIG).sort().join(', ')}`)
}

// MODE A: PROMPT
if (args.command === 'prompt') {
  const promptPath = join(rootDir, '.brain', 'msp', 'prompts', `${type}.prompt.md`)
  if (existsSync(promptPath)) {
    console.log(readFileSync(promptPath, 'utf8'))
  } else {
    // Fallback if codegen hasn't run
    let promptTemplate = `Creating a ${type.toUpperCase()} atom. Provide content for these sections:\n`
    for (let i = 0; i < config.sections.length; i++) {
      promptTemplate += `${i + 1}. ${config.sections[i]}:\n`
    }
    console.log(promptTemplate)
  }
  process.exit(0)
}

// For create and scaffold, slug and title are required
if (!args.slug) die('missing --slug (e.g. --slug=NEW-FEATURE)')

if (!/^[A-Z][A-Z0-9_-]*$/.test(args.slug)) {
  die(`invalid slug '${args.slug}' — must match ^[A-Z][A-Z0-9_-]*$ (use SCREAMING-KEBAB-CASE)`)
}
const slug = args.slug

// Compute paths
const filename = `${type.toUpperCase()}--${slug}.md`
const filepath = join(rootDir, 'gks', config.folder, filename)

if (existsSync(filepath) && !args.force) {
  die(`file already exists: ${filepath} (use --force to overwrite)`)
}

const now = new Date()
const shifted = new Date(now.getTime() + 7 * 3600 * 1000)
const isoTH = `${shifted.getUTCFullYear()}-${String(shifted.getUTCMonth() + 1).padStart(2, '0')}-${String(shifted.getUTCDate()).padStart(2, '0')}T${String(shifted.getUTCHours()).padStart(2, '0')}:${String(shifted.getUTCMinutes()).padStart(2, '0')}:${String(shifted.getUTCSeconds()).padStart(2, '0')}.${String(shifted.getUTCMilliseconds()).padStart(3, '0')}+07:00`

const id = `${type.toUpperCase()}--${slug}`
const title = args.title ?? slug.toLowerCase().split('-').map(w => w[0]!.toUpperCase() + w.slice(1)).join(' ')

// MODE B: CREATE (Token-optimal)
if (args.command === 'create') {
  if (!args['body-from']) die('missing --body-from (file path or - for stdin)')
  
  let bodyContent = ''
  if (args['body-from'] === '-') {
    bodyContent = readFileSync(0, 'utf-8')
  } else {
    bodyContent = readFileSync(resolve(args['body-from']), 'utf-8')
  }

  // Basic validation: ensure all required sections are present in the provided body
  for (const section of config.sections) {
    // Regex matches Markdown header format for the section
    const sectionRegex = new RegExp(`^##\\s+${section}\\s*$`, 'm')
    if (!sectionRegex.test(bodyContent)) {
      // It's possible the LLM used JSON if specified, but the prompt says "Markdown with section headers".
      // We will check if it's a JSON string.
      try {
        const parsedBody = JSON.parse(bodyContent)
        if (!parsedBody[section]) {
          die(`validation failed: missing section '${section}' in JSON body`)
        }
      } catch (e) {
         // Not JSON, and regex failed
         die(`validation failed: missing '## ${section}' in markdown body`)
      }
    }
  }

  // Convert JSON to Markdown if JSON was provided
  let finalBody = bodyContent
  try {
    const parsedBody = JSON.parse(bodyContent)
    finalBody = ''
    for (const section of config.sections) {
      finalBody += `## ${section}\n\n${parsedBody[section] || 'TODO'}\n\n`
    }
  } catch (e) {
    // It's already markdown
  }

  const aliases = buildAliases(id, undefined, rootDir)
  const aliasesText = aliases.map(a => `  - ${a}`).join('\n')
  const typeDef = lookupType(id.split('--')[0]!, rootDir)
  const clusterField = typeDef ? `cluster: ${typeDef.cluster}\n` : ''
  const roleField = typeDef ? `role: ${typeDef.role}\n` : ''

  let frontmatter = `---
id: ${id}
`
  if (config.db_id) {
    frontmatter += `${config.db_id}: ${id}\n`
  }
  frontmatter += `phase: ${config.phase}
type: ${type}
status: draft
tier: ${config.tier}
source_type: axiomatic
vault_id: default
title: ${title}
aliases:
${aliasesText}
${clusterField}${roleField}tags:
  - msp
crosslinks: {}
created_at: ${isoTH}
---

# ${type === 'audit' ? 'AUDIT' : type === 'proto' ? 'PROTO' : type === 'algo' ? 'ALGO' : type.toUpperCase()} — ${title}

`

  mkdirSync(dirname(filepath), { recursive: true })
  writeFileSync(filepath, frontmatter + finalBody)
  console.log(`✓ created ${filepath} (Token-Optimal Mode)`)
  process.exit(0)
}

// MODE C: SCAFFOLD (Legacy)
if (args.command === 'scaffold') {
  const aliases = buildAliases(id, undefined, rootDir)
  const aliasesText = aliases.map(a => `  - ${a}`).join('\n')
  const typeDef = lookupType(id.split('--')[0]!, rootDir)
  const clusterField = typeDef ? `cluster: ${typeDef.cluster}\n` : ''
  const roleField = typeDef ? `role: ${typeDef.role}\n` : ''

  let frontmatter = `---
id: ${id}
`
  if (config.db_id) {
    frontmatter += `${config.db_id}: ${id}\n`
  }
  frontmatter += `phase: ${config.phase}
type: ${type}
status: draft
tier: ${config.tier}
source_type: axiomatic
vault_id: default
title: ${title}
aliases:
${aliasesText}
${clusterField}${roleField}tags:
  - msp
crosslinks: {}
created_at: ${isoTH}
---

# ${type === 'audit' ? 'AUDIT' : type === 'proto' ? 'PROTO' : type === 'algo' ? 'ALGO' : type.toUpperCase()} — ${title}

`

  const body = config.sections.map((s: string) => `## ${s}\n\nTODO\n`).join('\n')

  mkdirSync(dirname(filepath), { recursive: true })
  writeFileSync(filepath, frontmatter + body)

  console.log(`✓ created ${filepath}`)
  console.log(`  next steps:`)
  console.log(`    1. fill in body sections (TODO markers)`)
  console.log(`    2. add crosslinks to related atoms`)
  console.log(`    3. npm run msp:validate ${filepath} --root=${rootDir}`)
  process.exit(0)
}
