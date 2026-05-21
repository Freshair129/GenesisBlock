#!/usr/bin/env node
import { writeFile, mkdir } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { MspNotionClient } from './client.js'
import { inferGksType, generateGksId } from './mapper.js'

const HELP = `msp-migrate notion — fetch and convert Notion pages into GKS atoms

Usage:
  msp-migrate notion --database-id <id> [--token <token>] [--out <dir>]
  msp-migrate notion --help

Flags:
  --database-id <id>  UUID of the source Notion database (required)
  --token <token>     Notion API token (default: env NOTION_TOKEN)
  --out <dir>         Output directory (default: gks/inbound)
  --help              This message

Example:
  msp-migrate notion --database-id abc-123 --out gks/inbound/legacy
`

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        'database-id': { type: 'string' },
        token: { type: 'string' },
        out: { type: 'string' },
        help: { type: 'boolean', short: 'h' },
      },
    })
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n${HELP}`)
    return 2
  }
  const { values } = parsed

  if (values.help || !values['database-id']) {
    process.stdout.write(HELP)
    return values.help ? 0 : 1
  }

  const token = values.token || process.env['NOTION_TOKEN']
  if (!token) {
    process.stderr.write('✗ Error: Notion API token not found. Use --token or set NOTION_TOKEN env var.\n')
    return 1
  }

  const databaseId = values['database-id']!
  const outDir = resolve(values.out || 'gks/inbound')
  const client = new MspNotionClient({ token })

  try {
    await mkdir(outDir, { recursive: true })
    process.stdout.write(`🔍 Querying Notion database ${databaseId}...\n`)
    
    const pages = await client.fetchDatabasePages(databaseId)
    process.stdout.write(`✓ Found ${pages.length} pages. Starting conversion...\n\n`)

    for (const page of pages) {
      const pageId = page.id
      const properties = (page as any).properties
      const title = properties['Name']?.title?.[0]?.plain_text || 'Untitled'
      
      const type = inferGksType(properties)
      const atomId = generateGksId(type, title)
      
      process.stdout.write(`▸ Processing [${atomId}]: ${title}...\n`)
      
      const mdBody = await client.pageToMarkdown(pageId)
      
      const content = `---
id: ${atomId}
phase: 1
type: ${type.toLowerCase()}
status: stub
vault_id: default
tier: process
source_type: axiomatic
title: ${title}
tags: [notion-import]
created_at: ${new Date().toISOString()}
---

# ${title}

${mdBody}

## Source
- Migrated from Notion page ID: ${pageId}
`

      const fileName = `${atomId}.md`
      await writeFile(join(outDir, fileName), content, 'utf8')
    }

    process.stdout.write(`\n✅ Migration complete! Atoms written to ${outDir}\n`)
    process.stdout.write('Remember to run `npm run msp:validate` on the imported files.\n')

  } catch (err) {
    process.stderr.write(`✗ Migration failed: ${(err as Error).message}\n`)
    return 1
  }

  return 0
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ fatal error: ${(err as Error).message}\n`)
    process.exit(2)
  })
