#!/usr/bin/env node
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'

import { parseRootFromArgv } from './argv.js'
import { createMspMcpServer } from './server.js'

async function main(): Promise<void> {
  const root = parseRootFromArgv(process.argv.slice(2))
  const server = createMspMcpServer({ root })
  const transport = new StdioServerTransport()
  await server.connect(transport)
}

main().catch((err) => {
  process.stderr.write(`✗ msp-mcp-server: ${(err as Error).message}\n`)
  process.exit(1)
})
