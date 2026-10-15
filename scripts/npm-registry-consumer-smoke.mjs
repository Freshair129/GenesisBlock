#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { access, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const packageName = '@freshair129/gks-genesis-block-native';
const packageVersion = process.env.PACKAGE_VERSION ?? '0.2.7';
assert.match(packageVersion, /^\d+\.\d+\.\d+(?:-[0-9A-Za-z][0-9A-Za-z.-]*)?$/, 'PACKAGE_VERSION must be a semver version');

const consumer = await mkdtemp(join(tmpdir(), 'genesisdb-npm-consumer-'));

function runNpm(args) {
  const executable = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  const result = spawnSync(executable, args, {
    cwd: consumer,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error('npm ' + args.join(' ') + ' failed with exit code ' + result.status);
}

try {
  runNpm(['init', '-y']);
  runNpm(['install', '--no-audit', '--no-fund', packageName + '@' + packageVersion]);

  const consumerRequire = createRequire(join(consumer, 'package.json'));
  const installedPackage = join(consumer, 'node_modules', '@freshair129', 'gks-genesis-block-native');
  const manifest = JSON.parse(await readFile(join(installedPackage, 'package.json'), 'utf8'));
  const cliRelativePath = typeof manifest.bin === 'string' ? manifest.bin : manifest.bin?.['genesisblock-mcp'];
  assert.ok(cliRelativePath, 'published package does not declare the genesisblock-mcp executable');
  const cliPath = resolve(installedPackage, cliRelativePath);
  await access(cliPath);

  const { Client } = await import(pathToFileURL(consumerRequire.resolve('@modelcontextprotocol/sdk/client/index.js')).href);
  const { StdioClientTransport } = await import(pathToFileURL(consumerRequire.resolve('@modelcontextprotocol/sdk/client/stdio.js')).href);
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [cliPath],
    env: { ...process.env, GENESIS_DB_PATH: join(consumer, 'mcp-consumer-db') },
  });
  const client = new Client({ name: 'registry-consumer', version: '1.0.0' }, { capabilities: {} });
  try {
    await client.connect(transport);
    const listed = await client.listTools();
    const names = new Set(listed.tools.map(({ name }) => name));
    for (const required of ['query_hql', 'query_ir', 'retrieve_tiered_context', 'add_knowledge', 'gks_knowledge_promote']) {
      assert.ok(names.has(required), 'published MCP CLI did not expose ' + required);
    }
    console.log('registry MCP handshake (' + packageVersion + '): ' + listed.tools.length + ' tools');
  } finally {
    await transport.close();
  }

  const nativeSmoke = [
    "const assert = require('node:assert/strict');",
    "const binding = require(process.env.GENESIS_PACKAGE_NAME);",
    "const db = binding.GenesisDatabase.open({ path: process.env.GENESIS_DB_PATH, vectorDim: 4 });",
    "(async () => {",
    "  const node = await db.addNode({ id: 'registry-npm-smoke', labels: ['DistributionSmoke'], embedding: [1, 0, 0, 0] });",
    "  assert.equal(node.id, 'registry-npm-smoke');",
    "  await db.saveState();",
    "  console.log('registry N-API round trip: ' + node.id);",
    "})().catch(error => { console.error(error); process.exitCode = 1; });",
  ].join('\n');
  const nativeResult = spawnSync(process.execPath, ['-e', nativeSmoke], {
    cwd: consumer,
    env: {
      ...process.env,
      GENESIS_PACKAGE_NAME: packageName,
      GENESIS_DB_PATH: join(consumer, 'native-consumer-db'),
    },
    stdio: 'inherit',
  });
  if (nativeResult.error) throw nativeResult.error;
  if (nativeResult.status !== 0) {
    throw new Error('registry N-API consumer failed with exit code ' + nativeResult.status);
  }
} finally {
  await rm(consumer, { recursive: true, force: true });
}
