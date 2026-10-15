#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
if (pkg.bin?.['genesisblock-mcp'] !== 'mcp/cli.js') {
  throw new Error('package.json must expose genesisblock-mcp -> mcp/cli.js');
}

const out = execFileSync('npm', ['pack', '--dry-run', '--json'], {
  encoding: 'utf8',
  env: { ...process.env, npm_config_ignore_scripts: 'true' },
});
const packed = JSON.parse(out)[0];
const files = new Set((packed.files ?? []).map((file) => file.path));
for (const required of ['mcp/cli.js', 'mcp/server.js', 'index.js', 'package.json']) {
  if (!files.has(required)) {
    throw new Error(`npm package payload is missing ${required}`);
  }
}

console.log(`✓ MCP npm payload includes CLI + server (${packed.filename})`);
