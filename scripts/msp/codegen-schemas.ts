#!/usr/bin/env tsx
/**
 * msp:codegen — Generate JSON schemas and prompt templates from atom_registry.yaml
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { parse } from 'yaml'

const rootDir = resolve(process.cwd())
const schemaPath = join(rootDir, 'atom_schema.yaml')

if (!existsSync(schemaPath)) {
  console.error(`codegen: missing ${schemaPath}`)
  process.exit(1)
}

const registry = parse(readFileSync(schemaPath, 'utf8'))
const schemasDir = join(rootDir, '.brain', 'msp', 'schemas')
const promptsDir = join(rootDir, '.brain', 'msp', 'prompts')

mkdirSync(schemasDir, { recursive: true })
mkdirSync(promptsDir, { recursive: true })

let typesProcessed = 0

const taxonomy = registry.taxonomy ?? registry.schema_config?.taxonomy
for (const cluster of Object.values(taxonomy.clusters) as any[]) {
  for (const [id, config] of Object.entries(cluster.types) as [string, any][]) {
    const typeKey = id.toLowerCase()
    
    // 1. Generate JSON Schema
    const schema = {
      $schema: 'http://json-schema.org/draft-07/schema#',
      title: `${id} Atom Body Schema`,
      type: 'object',
      properties: {} as Record<string, any>,
      required: config.sections,
      additionalProperties: false
    }

    // Treat each section as a required string field in the parsed body
    for (const section of config.sections) {
      schema.properties[section] = {
        type: 'string',
        description: `Markdown content for the '## ${section}' section.`
      }
    }

    const schemaPath = join(schemasDir, `${typeKey}.schema.json`)
    writeFileSync(schemaPath, JSON.stringify(schema, null, 2) + '\n')

    // 2. Generate Prompt Template
    let promptTemplate = `Creating a ${id} atom. Provide content for these sections:\n`
    for (let i = 0; i < config.sections.length; i++) {
      promptTemplate += `${i + 1}. ${config.sections[i]}:\n`
    }

    const promptPath = join(promptsDir, `${typeKey}.prompt.md`)
    writeFileSync(promptPath, promptTemplate)

    typesProcessed++
  }
}

console.log(`✓ codegen: wrote ${typesProcessed} schemas and templates to .brain/msp/`)
