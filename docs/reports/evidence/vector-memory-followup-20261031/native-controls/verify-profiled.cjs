'use strict';

const fs = require('node:fs');
const { performance } = require('node:perf_hooks');

const [addonPath, dbPath, resultPath] = process.argv.slice(2);
if (!addonPath || !dbPath || !resultPath) {
  throw new Error('expected addonPath, dbPath, resultPath');
}
if (!fs.existsSync(dbPath)) {
  throw new Error('database does not exist: ' + dbPath);
}

const started = performance.now();
function fail(message) {
  throw new Error(message);
}

async function main() {
  const native = require(addonPath);
  const openStarted = performance.now();
  const db = native.GenesisDatabase.open({
    path: dbPath,
    pageCacheMb: 32,
    readOnly: true,
    vectorDim: 4,
  });
  const openMs = performance.now() - openStarted;

  const queryStarted = performance.now();
  const rows = await db.executeHql('MATCH (n:Memory) RETURN n.id, n.prop.text');
  const queryMs = performance.now() - queryStarted;
  if (!Array.isArray(rows)) fail('HQL result is not an array');
  if (rows.length !== 2000) fail('expected 2000 Memory rows, got ' + rows.length);

  const byIndex = new Map();
  for (const row of rows) {
    const id = row && row['n.id'];
    const text = row && row['n.text'];
    if (typeof id !== 'string' || typeof text !== 'string') {
      fail('HQL row is missing id or text: ' + JSON.stringify(row));
    }
    const match = /^mem:(\d+)$/.exec(id);
    if (!match) fail('unexpected node id: ' + id);
    const index = Number(match[1]);
    if (!Number.isInteger(index) || index < 0 || index >= 2000) {
      fail('node index outside expected range: ' + id);
    }
    if (byIndex.has(index)) fail('duplicate node id: ' + id);
    const expectedText = 'observation ' + index + ': the user asked about journal segment retention and folding';
    if (text !== expectedText) fail('unexpected persisted text for ' + id);
    byIndex.set(index, { id, text });
  }
  if (byIndex.size !== 2000) fail('unique persisted node count mismatch: ' + byIndex.size);
  if (!byIndex.has(0) || !byIndex.has(1999)) fail('first or last node is missing');

  const result = {
    phase: 'read_only_reopen_verified',
    status: 'complete',
    node_version: process.version,
    engine: native.engineNameSync(),
    version: native.versionSync(),
    schema_version: native.schemaVersionSync(),
    db_path: dbPath,
    read_only: true,
    page_cache_mb: 32,
    vector_dim: 4,
    retention: 'default',
    query: 'MATCH (n:Memory) RETURN n.id, n.prop.text',
    query_row_count: rows.length,
    unique_node_count: byIndex.size,
    first_node: byIndex.get(0),
    last_node: byIndex.get(1999),
    open_ms: openMs,
    query_ms: queryMs,
    elapsed_ms: performance.now() - started,
    drop_measured: false,
  };
  fs.writeFileSync(resultPath, JSON.stringify(result, null, 2) + '\n');
  process.stdout.write('VERIFY_JSON ' + JSON.stringify(result) + '\n');
  process.exit(0);
}

main().catch((error) => {
  const failure = {
    phase: 'read_only_reopen_failed',
    status: 'failed',
    elapsed_ms: performance.now() - started,
    message: String(error && error.stack ? error.stack : error),
  };
  try {
    fs.writeFileSync(resultPath, JSON.stringify(failure, null, 2) + '\n');
    process.stderr.write(JSON.stringify(failure) + '\n');
  } finally {
    process.exit(2);
  }
});
