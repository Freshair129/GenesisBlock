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

test('schemaVersionSync matches modules.json engine.schemaVersion', () => {
  // The engine's on-disk format version must move in lock-step with the repo
  // version manifest — an accidental SCHEMA_VERSION bump (or a manifest left
  // behind) fails here. History: v2 = WP-1.2 framed journal; v3 = Slice-0
  // Event::NodeRetract journal frames (RCA--SLICE0-DURABILITY — older engines
  // silently skip unknown event kinds, so downgrade fails closed instead of
  // silently resurrecting deletions; ADR--GENESISDB-JOURNAL-HISTORY §4).
  // TypeScript-side consumers (packages/gks schema-version.ts) track the same
  // major byte (PROTOCOL--GENESIS-GRAPH-FFI §6).
  const manifest = JSON.parse(
    readFileSync(fileURLToPath(new URL('../modules.json', import.meta.url)), 'utf8'),
  )
  assert.equal(schemaVersionSync(), manifest.engine.schemaVersion)
  assert.equal(schemaVersionSync(), 3)
})
