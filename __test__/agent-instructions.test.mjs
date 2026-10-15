import assert from 'node:assert/strict';
import test from 'node:test';
import { readCanonicalPolicy, renderGeneratedPolicy } from '../scripts/sync-agent-instructions.mjs';

const canonical = `---
version: "1.0.0"
status: active
superseded_by: null
---
<!-- CANONICAL_AGENT_POLICY_START -->
${Array.from({ length: 10 }, (_, index) => `### R${index + 1} — Rule ${index + 1}`).join('\n')}
<!-- CANONICAL_AGENT_POLICY_END -->
`;

test('canonical policy requires active metadata and all R1-R10 headings', () => {
  const parsed = readCanonicalPolicy(canonical);
  assert.equal(parsed.version, '1.0.0');
  assert.match(parsed.policy, /^### R1 — Rule 1/m);
  assert.match(parsed.policy, /^### R10 — Rule 10/m);
});

test('canonical policy rejects a missing rule heading', () => {
  const missingR4 = canonical.replace('### R4 — Rule 4\n', '');
  assert.throws(() => readCanonicalPolicy(missingR4), /exactly one heading for R4; found 0/);
});

test('canonical policy rejects inactive or superseded metadata', () => {
  const inactive = canonical.replace('status: active', 'status: inactive');
  const superseded = canonical.replace('superseded_by: null', 'superseded_by: "2.0.0"');

  assert.throws(() => readCanonicalPolicy(inactive), /status: active/);
  assert.throws(() => readCanonicalPolicy(superseded), /superseded_by: null/);
});

test('generated policy refresh preserves tool-specific content outside markers', () => {
  const target = `# Tool instructions\nBefore\n<!-- GENERATED_AGENT_POLICY_START -->\nstale\n<!-- GENERATED_AGENT_POLICY_END -->\nAfter\n`;
  const parsed = readCanonicalPolicy(canonical);
  const { current, expected } = renderGeneratedPolicy(target, parsed);

  assert.equal(current, 'stale');
  assert.match(expected, /Before[\s\S]*Generated from AGENTS\.md v1\.0\.0/);
  assert.match(expected, /After\n$/);
  assert.doesNotMatch(expected, /\nstale\n/);
});

test('generated policy rejects malformed marker pairs', () => {
  const parsed = readCanonicalPolicy(canonical);
  assert.throws(
    () => renderGeneratedPolicy('# Missing markers\n', parsed),
    /must contain exactly one start and end marker/,
  );
});
