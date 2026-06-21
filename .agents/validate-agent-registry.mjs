import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const repoRoot = process.cwd();
const registryPath = path.join(repoRoot, '.agents', 'agent-registry.yaml');

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8').replace(/\r\n/g, '\n');
}

function fail(message) {
  throw new Error(message);
}

function exists(relPath) {
  return fs.existsSync(path.join(repoRoot, relPath));
}

function linesOf(sectionText) {
  return sectionText.split('\n').map((line) => line.replace(/\s+$/, ''));
}

function section(lines, startPattern, endPattern) {
  const start = lines.findIndex((line) => startPattern.test(line));
  if (start === -1) return [];
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (endPattern.test(lines[i])) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end);
}

function topLevelBlocks(sectionLines, headerPattern) {
  const headers = [];
  sectionLines.forEach((line, idx) => {
    if (headerPattern.test(line)) {
      headers.push(idx);
    }
  });
  const blocks = new Map();
  headers.forEach((startIdx, i) => {
    const endIdx = i + 1 < headers.length ? headers[i + 1] : sectionLines.length;
    const header = sectionLines[startIdx].trim().slice(0, -1);
    blocks.set(header, sectionLines.slice(startIdx, endIdx));
  });
  return blocks;
}

function collectList(sectionLines, keyName) {
  const idx = sectionLines.findIndex((line) => new RegExp(`^\\s{2}${keyName}:\\s*$`).test(line));
  if (idx === -1) return [];
  const items = [];
  for (let i = idx + 1; i < sectionLines.length; i += 1) {
    const line = sectionLines[i];
    if (/^\s{2}[a-zA-Z0-9_./-]+:\s*$/.test(line)) break;
    const m = line.match(/^\s{4}-\s+(.*)$/);
    if (m) items.push(m[1].trim());
  }
  return items;
}

function collectAgentBlocks(agentSectionLines) {
  const ids = [];
  const starts = [];
  agentSectionLines.forEach((line, idx) => {
    const match = line.match(/^\s{2}([a-z][a-z0-9_]*)\:\s*$/);
    if (match) {
      ids.push(match[1]);
      starts.push(idx);
    }
  });
  const blocks = new Map();
  starts.forEach((startIdx, i) => {
    const endIdx = i + 1 < starts.length ? starts[i + 1] : agentSectionLines.length;
    blocks.set(ids[i], agentSectionLines.slice(startIdx, endIdx));
  });
  return blocks;
}

function valueFrom(blockLines, key) {
  const match = blockLines.find((line) => new RegExp(`^\\s{4}${key}:\\s*(.*)$`).test(line));
  if (!match) return null;
  return match.replace(new RegExp(`^\\s{4}${key}:\\s*`), '').trim();
}

function listFrom(blockLines, key) {
  const idx = blockLines.findIndex((line) => new RegExp(`^\\s{4}${key}:\\s*$`).test(line));
  if (idx === -1) return [];
  const out = [];
  for (let i = idx + 1; i < blockLines.length; i += 1) {
    const line = blockLines[i];
    if (/^\s{4}[a-zA-Z0-9_./-]+:\s*$/.test(line)) break;
    const match = line.match(/^\s{6}-\s+(.*)$/);
    if (match) out.push(match[1].trim());
  }
  return out;
}

const text = readText(registryPath);
if (text.includes('<path>') || text.includes('...')) {
  fail('Registry still contains placeholder tokens like <path> or ...');
}

const lines = linesOf(text);
const rootSection = section(lines, /^root_contract:\s*$/, /^executor_defaults:\s*$/);
const agentsSection = section(lines, /^agents:\s*$/, /^routes:\s*$/);
const routesStart = lines.findIndex((line) => /^routes:\s*$/.test(line));
const routesSection = routesStart === -1 ? [] : lines.slice(routesStart + 1);

const globalContext = collectList(rootSection, 'global_context');
const requiredGlobal = [
  'setup/AGENT.md',
  'AGENT.md',
  'ARCHITECTURE.md',
  'CONTRIBUTING.md',
  'README.md',
  'docs/C4--GENESISDB-ARCHITECTURE.md',
  'docs/MASTER-SPEC--GENESIS-DB.md',
  '.agents/agent-registry.yaml',
  '.agents/validate-agent-registry.mjs',
];
for (const relPath of requiredGlobal) {
  if (!globalContext.includes(relPath)) {
    fail(`Missing global_context entry: ${relPath}`);
  }
  if (!exists(relPath)) {
    fail(`Missing file referenced by global_context: ${relPath}`);
  }
}

const blocks = collectAgentBlocks(agentsSection);
const expectedAgents = ['origin', 'lyra', 'rusty', 'genesis', 'ather', 'kin'];
for (const agentId of expectedAgents) {
  if (!blocks.has(agentId)) {
    fail(`Missing agent block: ${agentId}`);
  }
}

const seenAgentIds = new Set();
const seenLabels = new Set();
const agentScopes = new Map();

for (const [agentId, blockLines] of blocks.entries()) {
  const stableId = valueFrom(blockLines, 'agent_id');
  const label = valueFrom(blockLines, 'label');
  const role = valueFrom(blockLines, 'role');
  const persona = valueFrom(blockLines, 'persona');
  const contract = valueFrom(blockLines, 'contract');
  const scope = listFrom(blockLines, 'scope');
  const asset = listFrom(blockLines, 'asset');
  const context = listFrom(blockLines, 'context');
  const allowedScopes = listFrom(blockLines, 'allowed_scopes');
  const defaultContext = listFrom(blockLines, 'default_context');

  if (!stableId) fail(`Missing agent_id for ${agentId}`);
  if (stableId !== `agt_${agentId}`) fail(`agent_id drift for ${agentId}: expected agt_${agentId}, got ${stableId}`);
  if (!/^([A-Z][A-Za-z0-9 ]*) \([^)]+\)$/.test(label || '')) fail(`Label format must be English (ไทย) for ${agentId}`);
  if (!role) fail(`Missing role for ${agentId}`);
  if (!persona) fail(`Missing persona for ${agentId}`);
  if (!contract) fail(`Missing contract for ${agentId}`);
  if (!exists(contract)) fail(`Contract path does not exist for ${agentId}: ${contract}`);
  if (scope.length === 0) fail(`Missing scope entries for ${agentId}`);
  if (asset.length === 0) fail(`Missing asset entries for ${agentId}`);
  if (context.length === 0) fail(`Missing context entries for ${agentId}`);
  if (allowedScopes.length === 0) fail(`Missing allowed_scopes entries for ${agentId}`);
  if (defaultContext.length === 0) fail(`Missing default_context entries for ${agentId}`);

  for (const entry of scope) {
    if (!allowedScopes.includes(entry)) {
      fail(`Scope entry must be mirrored in allowed_scopes for ${agentId}: ${entry}`);
    }
  }

  for (const relPath of [...asset, ...context, ...defaultContext]) {
    if (!exists(relPath)) {
      fail(`Referenced path does not exist for ${agentId}: ${relPath}`);
    }
  }

  if (seenAgentIds.has(stableId)) fail(`Duplicate agent_id: ${stableId}`);
  if (seenLabels.has(label)) fail(`Duplicate label: ${label}`);
  seenAgentIds.add(stableId);
  seenLabels.add(label);
  agentScopes.set(agentId, allowedScopes);
}

const routeBlocks = topLevelBlocks(routesSection, /^\s{2}[A-Za-z0-9_./-]+:\s*$/);

for (const [routeKey, routeBlock] of routeBlocks.entries()) {
  const preferred = valueFrom(routeBlock, 'preferred_agent');
  if (!preferred) fail(`Missing preferred_agent for route ${routeKey}`);
  if (!blocks.has(preferred)) fail(`Route ${routeKey} points to unknown agent: ${preferred}`);
  const preferredScopes = agentScopes.get(preferred) || [];
  const routeAllowed = preferredScopes.some((scope) => routeKey === scope || routeKey.startsWith(scope));
  if (!routeAllowed) {
    fail(`Route ${routeKey} is not inside allowed_scopes for preferred agent ${preferred}`);
  }
  const reviewers = listFrom(routeBlock, 'reviewers');
  for (const reviewer of reviewers) {
    if (!blocks.has(reviewer)) {
      fail(`Route ${routeKey} has unknown reviewer agent: ${reviewer}`);
    }
  }
}

console.log(`agent-registry ok: ${expectedAgents.length} agents, ${routeBlocks.size} routes`);
