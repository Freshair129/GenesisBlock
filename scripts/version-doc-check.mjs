#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const CARGO = join(ROOT, 'Cargo.toml');
const VERSION_DOC = join(ROOT, 'docs', 'VERSION.md');

function cargoVersion() {
  const text = readFileSync(CARGO, 'utf8');
  const pkg = text.includes('[package]') ? text.split('[package]', 2)[1] : text;
  const match = /^version\s*=\s*"([^"]+)"/m.exec(pkg);
  if (!match) throw new Error('could not read [package] version from Cargo.toml');
  return match[1];
}

function documentedVersion() {
  const text = readFileSync(VERSION_DOC, 'utf8');
  const match = /\| \*\*Engine crate\*\*[^\n]*\|\s*`([^`]+)`\s*\|/.exec(text);
  if (!match) throw new Error('could not read Engine crate version from docs/VERSION.md');
  return match[1];
}

try {
  const cargo = cargoVersion();
  const doc = documentedVersion();
  if (cargo !== doc) {
    console.error('✗ docs/VERSION.md drift detected:');
    console.error(`    Cargo.toml [package].version : ${cargo}`);
    console.error(`    docs/VERSION.md Engine crate : ${doc}`);
    process.exit(1);
  }
  console.log(`✓ docs/VERSION.md agrees on version ${cargo}`);
} catch (error) {
  console.error(`✗ ${error.message}`);
  process.exit(1);
}
