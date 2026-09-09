import process from 'node:process';
import { GenesisRag17Worker } from './worker.mjs';

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name}_REQUIRED`);
  return value;
}

const worker = GenesisRag17Worker.create({
  dbPath: required('GENESIS_WORKER_DB_PATH'),
  scope: JSON.parse(required('GENESIS_WORKER_SCOPE')),
  credential: required('GENESIS_WORKER_CREDENTIAL'),
  workerToken: required('GENESIS_WORKER_QUERY_TOKEN'),
  modelDir: required('GENESIS_WORKER_MODEL_DIR'),
  benchmarkFixture: required('GENESIS_WORKER_BENCHMARK_FIXTURE'),
  pythonCommand: process.env.GENESISRAG17_PYTHON,
  mspCommand: {
    command: required('GENESIS_WORKER_MSP_COMMAND'),
    args: process.env.GENESIS_WORKER_MSP_ARGS ? JSON.parse(process.env.GENESIS_WORKER_MSP_ARGS) : [],
    cwd: process.env.GENESIS_WORKER_MSP_CWD,
    timeoutMs: Number(process.env.GENESIS_WORKER_MSP_TIMEOUT_MS ?? 30000),
  },
  host: '127.0.0.1',
  port: Number(process.env.GENESIS_WORKER_PORT ?? 0),
  pollIntervalMs: Number(process.env.GENESIS_WORKER_POLL_MS ?? 1000),
});

const endpoint = await worker.listen();
process.stdout.write(`${JSON.stringify(endpoint)}\n`);
worker.resume();

const shutdown = async () => {
  await worker.close();
  process.exit(0);
};
process.once('SIGINT', shutdown);
process.once('SIGTERM', shutdown);
