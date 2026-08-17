import { test } from 'node:test'
import assert from 'node:assert/strict'

// index.js is CommonJS — default import gives us the whole module exports
// object, which we destructure here. Avoids the Node 20 ESM static-analysis
// gotcha with native bindings (the names are filled in at runtime).
import binding from '../index.js'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const { schemaVersionSync, engineNameSync, versionSync } = binding

test('engineNameSync returns the stable identifier', () => {
  assert.equal(engineNameSync(), 'genesis-block')
})

test('versionSync matches package.json version (single source of truth)', () => {
  const pkg = JSON.parse(
    readFileSync(fileURLToPath(new URL('../package.json', import.meta.url)), 'utf8'),
  )
  // The Rust ENGINE_VERSION is env!("CARGO_PKG_VERSION"); the version CLI keeps
  // Cargo.toml and package.json in lock-step, so these must be equal.
  assert.equal(versionSync(), pkg.version)
})

test('schemaVersionSync matches the TypeScript-side CURRENT_SCHEMA_VERSION major byte', () => {
  // packages/gks/src/lib/schema-version.ts → CURRENT_SCHEMA_VERSION = '2.0.0'
  // The Rust crate encodes only the major byte (PROTOCOL--GENESIS-GRAPH-FFI §6).
  // Bumped to 2 by WP-1.2 (framed journal): the on-disk format changed, so a
  // client built against v1 must not silently open a migrated database
  // (ADR--GENESISDB-JOURNAL-HISTORY §4 — downgrade fails closed).
  assert.equal(schemaVersionSync(), 2)
})
