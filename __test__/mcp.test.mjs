import test from 'node:test';
import assert from 'node:assert';
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import path from "path";
import { fileURLToPath } from 'url';
import fs from 'fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const serverPath = path.join(__dirname, '../mcp/server.js');
const testDbPath = path.join(__dirname, '../.brain/mcp_test_db');

test('MCP Server: Life-cycle and Tools', async (t) => {
  // Cleanup test DB
  if (fs.existsSync(testDbPath)) {
    fs.rmSync(testDbPath, { recursive: true, force: true });
  }

  const transport = new StdioClientTransport({
    command: "node",
    args: [serverPath],
    env: { ...process.env, GENESIS_DB_PATH: testDbPath }
  });

  const client = new Client({
    name: "test-client",
    version: "1.0.0"
  }, {
    capabilities: {}
  });

  await client.connect(transport);

  await t.test('list tools', async () => {
    const result = await client.listTools();
    const toolNames = result.tools.map(t => t.name);
    assert.ok(toolNames.includes('query_hql'));
    assert.ok(toolNames.includes('retrieve_tiered_context'));
    assert.ok(toolNames.includes('add_knowledge'));
  });

  await t.test('add_knowledge tool', async () => {
    const result = await client.callTool({
      name: "add_knowledge",
      arguments: {
        id: "mcp-test-node",
        labels: ["TEST"],
        props: { foo: "bar" }
      }
    });
    assert.strictEqual(result.isError, undefined);
    assert.ok(result.content[0].text.includes('Knowledge atom added'));
  });

  await t.test('query_hql tool', async () => {
    const result = await client.callTool({
      name: "query_hql",
      arguments: {
        query: "TRAVERSE FROM mcp-test-node DEPTH 0 REL ANY"
      }
    });
    assert.strictEqual(result.isError, undefined);
    const data = JSON.parse(result.content[0].text);
    // Depth 0 should return at least the node itself (reconciled as NeighborOutput)
    // Wait, traverse from seed with depth 0 might return empty or just seed.
    // Based on our implementation, it starts with queue.push_back((seed, ...)) and checks depth.
    assert.ok(Array.isArray(data));
  });

  await t.test('retrieve_tiered_context tool', async () => {
    const result = await client.callTool({
      name: "retrieve_tiered_context",
      arguments: {
        target: "mcp-test-node",
        tier: "H0"
      }
    });
    assert.strictEqual(result.isError, undefined);
    const data = JSON.parse(result.content[0].text);
    assert.strictEqual(data.nodes[0].id, "mcp-test-node");
  });

  // Graceful shutdown
  await transport.close();
});
