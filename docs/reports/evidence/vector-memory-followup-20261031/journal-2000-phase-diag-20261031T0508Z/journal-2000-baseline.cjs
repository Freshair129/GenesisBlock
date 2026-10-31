'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');

const addonPath = process.argv[2];
const dbPath = process.argv[3];
const progressPath = process.argv[4];
const phasesPath = process.argv[5];
if (!addonPath || !dbPath || !progressPath || !phasesPath) {
  throw new Error('expected addonPath, dbPath, progressPath, phasesPath');
}
if (fs.existsSync(dbPath)) {
  throw new Error('refusing to overwrite existing database: ' + dbPath);
}
if (typeof global.gc !== 'function') {
  throw new Error('run node with --expose-gc so native Storage Drop can be measured');
}

const {
  GenesisDatabase,
  engineNameSync,
  versionSync,
  schemaVersionSync,
} = require(addonPath);

const started = performance.now();
let db = null;
let resolveNativeFinalizer;
const nativeFinalized = new Promise((resolve) => {
  resolveNativeFinalizer = resolve;
});
const registry = new FinalizationRegistry(() => {
  const elapsedMs = performance.now() - started;
  const event = {
    phase: 'native_wrapper_finalized',
    elapsed_ms: elapsedMs,
    note: 'FinalizationRegistry callback; verify Storage Drop marker in stdout log',
  };
  fs.appendFileSync(phasesPath, JSON.stringify(event) + '\n');
  process.stdout.write(JSON.stringify(event) + '\n');
  resolveNativeFinalizer(elapsedMs);
});

function elapsedMs() {
  return performance.now() - started;
}
function emit(phase, values) {
  const event = Object.assign({
    phase,
    elapsed_ms: elapsedMs(),
    unix_time_ms: Date.now(),
  }, values || {});
  fs.appendFileSync(phasesPath, JSON.stringify(event) + '\n');
  fs.appendFileSync(progressPath, JSON.stringify(event) + '\n');
  process.stdout.write(JSON.stringify(event) + '\n');
  return event;
}

async function main() {
  const openStarted = performance.now();
  db = GenesisDatabase.open({
    path: dbPath,
    pageCacheMb: 32,
    readOnly: false,
    vectorDim: 4,
  });
  const openMs = performance.now() - openStarted;
  const engine = {
    engine: engineNameSync(),
    version: versionSync(),
    schema_version: schemaVersionSync(),
  };
  registry.register(db, 'storage');
  emit('opened', Object.assign({ open_ms: openMs, db_path: dbPath }, engine));

  const addStarted = performance.now();
  let batchStarted = performance.now();
  for (let i = 0; i < 2000; i++) {
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
    if ((i + 1) % 250 === 0) {
      const now = performance.now();
      emit('add_progress', {
        added: i + 1,
        total: 2000,
        add_elapsed_ms: now - addStarted,
        latest_250_elapsed_ms: now - batchStarted,
      });
      batchStarted = now;
    }
  }
  const addElapsedMs = performance.now() - addStarted;
  emit('add_complete', { added: 2000, add_elapsed_ms: addElapsedMs });

  const saveStarted = performance.now();
  await db.saveState();
  const saveElapsedMs = performance.now() - saveStarted;
  const saveDoneElapsedMs = elapsedMs();
  emit('explicit_save_done', {
    save_elapsed_ms: saveElapsedMs,
    save_done_elapsed_ms: saveDoneElapsedMs,
  });

  const dropStartedElapsedMs = elapsedMs();
  emit('drop_start', {
    save_elapsed_ms: saveElapsedMs,
    save_done_elapsed_ms: saveDoneElapsedMs,
  });
  db = null;
  global.gc();
  const finalizedElapsedMs = await nativeFinalized;
  const dropWaitMs = finalizedElapsedMs - dropStartedElapsedMs;
  const final = emit('drop_done', {
    drop_wait_ms: dropWaitMs,
    explicit_save_elapsed_ms: saveElapsedMs,
    save_done_elapsed_ms: saveDoneElapsedMs,
    total_elapsed_ms: elapsedMs(),
    storage_drop_marker_expected: 'shutdown: graceful shutdown, state saved.',
    status: 'complete',
  });
  process.stdout.write('FINAL_JSON ' + JSON.stringify(final) + '\n');
  process.exit(0);
}

main().catch((error) => {
  const failure = {
    phase: 'fatal_error',
    elapsed_ms: elapsedMs(),
    message: String(error && error.stack ? error.stack : error),
  };
  try {
    fs.appendFileSync(phasesPath, JSON.stringify(failure) + '\n');
    fs.appendFileSync(progressPath, JSON.stringify(failure) + '\n');
    process.stderr.write(JSON.stringify(failure) + '\n');
  } finally {
    process.exit(2);
  }
});
