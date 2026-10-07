import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const canonicalPath = path.join(repoRoot, 'AGENTS.md');
const sourceStart = '<!-- CANONICAL_AGENT_POLICY_START -->';
const sourceEnd = '<!-- CANONICAL_AGENT_POLICY_END -->';
const mirrorStart = '<!-- GENERATED_AGENT_POLICY_START -->';
const mirrorEnd = '<!-- GENERATED_AGENT_POLICY_END -->';
const mirrorPaths = ['CLAUDE.md', 'GEMINI.md'];
const ruleIds = Array.from({ length: 10 }, (_, index) => `R${index + 1}`);

function normalizeNewlines(text) {
  return text.replace(/\r\n/g, '\n');
}

function extractSingleBlock(text, startMarker, endMarker, label) {
  if (text.split(startMarker).length !== 2 || text.split(endMarker).length !== 2) {
    throw new Error(`${label} must contain exactly one start and end marker.`);
  }

  const startIndex = text.indexOf(startMarker) + startMarker.length;
  const endIndex = text.indexOf(endMarker);
  if (endIndex < startIndex) {
    throw new Error(`${label} markers are out of order.`);
  }

  return text.slice(startIndex, endIndex).trim();
}

export function readCanonicalPolicy(sourceText) {
  const source = normalizeNewlines(sourceText);
  const frontmatter = source.match(/^---\n([\s\S]*?)\n---(?:\n|$)/);
  if (!frontmatter) throw new Error('AGENTS.md must start with YAML frontmatter.');

  const metadata = frontmatter[1];
  const version = metadata.match(/^version:\s*["']?([^"'\n]+)["']?\s*$/m)?.[1];
  if (!version) throw new Error('AGENTS.md frontmatter must declare version.');
  if (!/^status:\s*active\s*$/m.test(metadata)) {
    throw new Error('The canonical AGENTS.md must have status: active.');
  }
  if (!/^superseded_by:\s*null\s*$/m.test(metadata)) {
    throw new Error('The active canonical AGENTS.md must have superseded_by: null.');
  }

  const policy = extractSingleBlock(source, sourceStart, sourceEnd, 'AGENTS.md');
  for (const ruleId of ruleIds) {
    const heading = new RegExp(`^### ${ruleId}(?:\\s|$)`, 'gm');
    const count = [...policy.matchAll(heading)].length;
    if (count !== 1) {
      throw new Error(`AGENTS.md policy must contain exactly one heading for ${ruleId}; found ${count}.`);
    }
  }

  return { version, policy };
}

export function renderGeneratedPolicy(targetText, { version, policy }) {
  const target = normalizeNewlines(targetText);
  const current = extractSingleBlock(target, mirrorStart, mirrorEnd, 'Generated mirror');
  const startIndex = target.indexOf(mirrorStart) + mirrorStart.length;
  const endIndex = target.indexOf(mirrorEnd);
  const generated = [
    '',
    `<!-- Generated from AGENTS.md v${version}; do not edit this section. -->`,
    '',
    policy.trim(),
    '',
  ].join('\n');
  const expected = `${target.slice(0, startIndex)}${generated}${target.slice(endIndex)}`;

  return { current, expected };
}

function preserveLineEndings(original, normalized) {
  return original.includes('\r\n') ? normalized.replace(/\n/g, '\r\n') : normalized;
}

function run() {
  const args = process.argv.slice(2);
  const checkOnly = args.includes('--check');
  if (args.some((arg) => arg !== '--check')) {
    throw new Error('Usage: node scripts/sync-agent-instructions.mjs [--check]');
  }

  const canonical = readCanonicalPolicy(fs.readFileSync(canonicalPath, 'utf8'));
  const outputs = mirrorPaths.map((relativePath) => {
    const absolutePath = path.join(repoRoot, relativePath);
    const original = fs.readFileSync(absolutePath, 'utf8');
    const { expected } = renderGeneratedPolicy(original, canonical);
    const finalText = preserveLineEndings(original, expected);
    return { absolutePath, finalText, original, relativePath };
  });
  const stalePaths = outputs.filter(({ original, finalText }) => original !== finalText).map(({ relativePath }) => relativePath);

  if (stalePaths.length > 0 && checkOnly) {
    throw new Error(`Generated agent policy is stale: ${stalePaths.join(', ')}. Run npm run agents:sync.`);
  }

  for (const { absolutePath, finalText, original, relativePath } of outputs) {
    if (original === finalText) continue;
    fs.writeFileSync(absolutePath, finalText, 'utf8');
    console.log(`Updated ${relativePath} from AGENTS.md v${canonical.version}.`);
  }
  if (checkOnly) console.log(`Agent policy mirrors match AGENTS.md v${canonical.version}.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    run();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
