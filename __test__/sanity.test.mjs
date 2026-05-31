import { test } from 'node:test'
import assert from 'node:assert/strict'

// index.js is CommonJS — default import gives us the whole module exports
// object, which we destructure here. Avoids the Node 20 ESM static-analysis
// gotcha with native bindings (the names are filled in at runtime).
import binding from '../index.js'

const { schemaVersionSync, engineNameSync } = binding

test('engineNameSync returns the stable identifier', () => {
  assert.equal(engineNameSync(), 'genesis-block')
})

test('schemaVersionSync matches the TypeScript-side CURRENT_SCHEMA_VERSION major byte', () => {
  // packages/gks/src/lib/schema-version.ts → CURRENT_SCHEMA_VERSION = '1.0.0'
  // The Rust crate encodes only the major byte (PROTOCOL--GENESIS-GRAPH-FFI §6).
  assert.equal(schemaVersionSync(), 1)
})
