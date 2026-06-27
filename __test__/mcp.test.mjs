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

  await t.test('list tools returns all three tools', async () => {
    const result = await client.listTools();
    const toolNames = result.tools.map(t => t.name);
    assert.ok(toolNames.includes('query_hql'), 'query_hql tool must be listed');
    assert.ok(toolNames.includes('retrieve_tiered_context'), 'retrieve_tiered_context tool must be listed');
    assert.ok(toolNames.includes('add_knowledge'), 'add_knowledge tool must be listed');
    assert.strictEqual(result.tools.length, 3, 'exactly 3 tools should be listed');
  });

  await t.test('add_knowledge with explicit ID', async () => {
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
    assert.ok(result.content[0].text.includes('mcp-test-node'), 'response should contain the node ID');
  });

  await t.test('add_knowledge generates ID when omitted', async () => {
    const result = await client.callTool({
      name: "add_knowledge",
      arguments: {
        labels: ["AUTO_ID"],
        props: { auto: true }
      }
    });
    assert.strictEqual(result.isError, undefined);
    assert.ok(result.content[0].text.includes('Knowledge atom added'));
  });

  await t.test('query_hql traverses known node', async () => {
    const result = await client.callTool({
      name: "query_hql",
      arguments: {
        query: 'TRAVERSE FROM "mcp-test-node" DEPTH 1 REL ANY'
      }
    });
    assert.strictEqual(result.isError, undefined);
    const data = JSON.parse(result.content[0].text);
    assert.ok(Array.isArray(data), 'TRAVERSE result must be an array');
  });

  await t.test('retrieve_tiered_context H0 returns target node', async () => {
    const result = await client.callTool({
      name: "retrieve_tiered_context",
      arguments: {
        target: "mcp-test-node",
        tier: "H0"
      }
    });
    assert.strictEqual(result.isError, undefined);
    const data = JSON.parse(result.content[0].text);
    assert.ok(data.nodes, 'context package must have nodes array');
    assert.strictEqual(data.nodes[0].id, "mcp-test-node", 'H0 context must include the target node');
  });

  await t.test('retrieve_tiered_context H1 returns context package shape', async () => {
    const result = await client.callTool({
      name: "retrieve_tiered_context",
      arguments: {
        target: "mcp-test-node",
        tier: "H1",
        budget: 1000
      }
    });
    assert.strictEqual(result.isError, undefined);
    const data = JSON.parse(result.content[0].text);
    assert.ok(data.nodes, 'context package must have nodes');
    assert.ok(data.edges !== undefined, 'context package must have edges');
    assert.ok(typeof data.tokenEstimate === 'number', 'tokenEstimate must be a number');
    assert.ok(typeof data.reasoningPath === 'string', 'reasoningPath must be a string');
  });

  await t.test('query_hql error path: malformed HQL returns structured error', async () => {
    const result = await client.callTool({
      name: "query_hql",
      arguments: {
        query: "NOT VALID HQL $$$$"
      }
    });
    assert.strictEqual(result.isError, true, "malformed HQL must return isError:true");
    const msg = result.content[0].text;
    assert.ok(typeof msg === 'string' && msg.length > 0, "error message must be a non-empty string");
  });

  await t.test('add_knowledge missing labels returns error', async () => {
    const result = await client.callTool({
      name: "add_knowledge",
      arguments: {}
    });
    // Missing required field 'labels' should produce an error
    assert.strictEqual(result.isError, true, "missing labels must return isError:true");
  });

  // Graceful shutdown
  await transport.close();
});
