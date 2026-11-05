'use strict';

const fs = require('node:fs');
const { performance } = require('node:perf_hooks');

const [addonPath, dbPath, phasePath, resultPath] = process.argv.slice(2);
if (!addonPath || !dbPath || !phasePath || !resultPath) {
  throw new Error('expected addonPath, dbPath, phasePath, resultPath');
}
if (fs.existsSync(dbPath)) {
  throw new Error('refusing to overwrite existing database: ' + dbPath);
}

const started = performance.now();
const serializationErrors = [];
function elapsedMs() {
  return performance.now() - started;
}
function writeJsonLine(file, value) {
  fs.appendFileSync(file, JSON.stringify(value) + '\n');
}
function emit(phase, values = {}) {
  const event = Object.assign({
    phase,
    elapsed_ms: elapsedMs(),
    unix_time_ms: Date.now(),
  }, values);
  writeJsonLine(phasePath, event);
  process.stdout.write(JSON.stringify(event) + '\n');
  return event;
}
function errorText(error) {
  if (error && error.stack) return String(error.stack);
  return String(error);
}
function observeSerializationError(phase, error) {
  const message = errorText(error);
  if (/(serializ|serde|json|encoding|encode|decode)/i.test(message)) {
    serializationErrors.push({ phase, message });
  }
}

async function main() {
  const requireStarted = performance.now();
  const native = require(addonPath);
  const requireMs = performance.now() - requireStarted;

  const openStarted = performance.now();
  const db = native.GenesisDatabase.open({
    path: dbPath,
    pageCacheMb: 32,
    readOnly: false,
    vectorDim: 4,
  });
  const openMs = performance.now() - openStarted;
  const opened = {
    phase: 'opened',
    elapsed_ms: elapsedMs(),
    unix_time_ms: Date.now(),
    node_version: process.version,
    engine: native.engineNameSync(),
    version: native.versionSync(),
    schema_version: native.schemaVersionSync(),
    require_ms: requireMs,
    open_ms: openMs,
    db_path: dbPath,
    read_only: false,
    page_cache_mb: 32,
    vector_dim: 4,
    retention: 'default',
    workload: '2000 individual addNode calls; no embedding',
    frontier_after_open: db.stableFrontier(),
  };
  writeJsonLine(phasePath, opened);
  process.stdout.write(JSON.stringify(opened) + '\n');

  const addStarted = performance.now();
  let groupStarted = performance.now();
  for (let i = 0; i < 2000; i++) {
    try {
      await db.addNode({
        id: 'mem:' + i,
        labels: ['Memory', 'Episodic'],
        props: {
          text: 'observation ' + i + ': the user asked about journal segment retention and folding',
          source: 'session-2026-08-17',
          confidence: 0.87,
          tags: ['journal', 'retention', 'wp-1.2'],
        },
        embedding: undefined,
        lang: 'en',
      });
    } catch (error) {
      observeSerializationError('addNode:' + i, error);
      throw error;
    }
    if ((i + 1) % 250 === 0) {
      const now = performance.now();
      emit('add_progress', {
        added: i + 1,
        total: 2000,
        add_elapsed_ms: now - addStarted,
        latest_250_elapsed_ms: now - groupStarted,
      });
      groupStarted = now;
    }
  }
  const ingestElapsedMs = performance.now() - addStarted;
  const frontierAfterIngest = db.stableFrontier();
  emit('ingest_complete', {
    added: 2000,
    ingest_elapsed_ms: ingestElapsedMs,
    frontier_after_ingest: frontierAfterIngest,
  });

  const saveStarted = performance.now();
  emit('save_start', { frontier_before_save: frontierAfterIngest });
  try {
    await db.saveState();
  } catch (error) {
    observeSerializationError('saveState', error);
    throw error;
  }
  const checkpointElapsedMs = performance.now() - saveStarted;
  const frontierAfterSave = db.stableFrontier();
  const final = {
    phase: 'complete',
    elapsed_ms: elapsedMs(),
    unix_time_ms: Date.now(),
    node_version: process.version,
    engine: native.engineNameSync(),
    version: native.versionSync(),
    schema_version: native.schemaVersionSync(),
    node_count_written: 2000,
    require_ms: requireMs,
    open_ms: openMs,
    ingest_elapsed_ms: ingestElapsedMs,
    explicit_checkpoint_elapsed_ms: checkpointElapsedMs,
    frontier_after_ingest: frontierAfterIngest,
    frontier_after_save: frontierAfterSave,
    serialization_error_observations: serializationErrors,
    db_path: dbPath,
    status: 'complete',
    drop_measured: false,
  };
  fs.writeFileSync(resultPath, JSON.stringify(final, null, 2) + '\n');
  writeJsonLine(phasePath, final);
  process.stdout.write('RESULT_JSON ' + JSON.stringify(final) + '\n');
  process.exit(0);
}

main().catch((error) => {
  const message = errorText(error);
  observeSerializationError('fatal', error);
  const failure = {
    phase: 'fatal_error',
    elapsed_ms: elapsedMs(),
    unix_time_ms: Date.now(),
    message,
    serialization_error_observations: serializationErrors,
    status: 'failed',
  };
  try {
    writeJsonLine(phasePath, failure);
    fs.writeFileSync(resultPath, JSON.stringify(failure, null, 2) + '\n');
    process.stderr.write(JSON.stringify(failure) + '\n');
  } finally {
    process.exit(2);
  }
});
