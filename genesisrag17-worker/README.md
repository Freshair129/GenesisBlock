# GenesisRAG17 Tier 4 worker

This package is the isolated **TEST** worker for stages 13, 15 and 16 of the
GenesisRAG17 pipeline. It owns one GenesisBlock native store process, receives
graph decisions and receipts through the MSP tool boundary, and exposes an
authenticated loopback query endpoint. It never opens the GKS store directly,
does not read the Edge store, and has no LLM-assisted extraction path.

The wire contract is
[`docs/plans/GENESISRAG17-CONTRACT.md`](../../zuri-ai-ki17/docs/plans/GENESISRAG17-CONTRACT.md)
(`genesisrag17.v1`, currently `1.2.0b`). The worker persists source, parsed
artifact, chunk, mention, entity, fact, held-fact, derived-object and citation
lineage in the native graph/SQLite projections before it reports a receipt.
Candidate snapshots are prepared on disk but are queryable only after the
atomic published pointer names their snapshot id. Previous published snapshot
ids remain queryable for correction and historical tests.

## Reproducible TEST setup

Use an x64 Windows Node.js `24.18.x` runtime for this package. The shared test
runtime used for the native and SQLite bindings is:

```powershell
$Ki17Runtime = 'C:\Users\pc\workspace\ki17-runtime'
$env:Path = "$Ki17Runtime;$env:Path"
node --version                 # v24.18.0

Set-Location 'C:\Users\pc\workspace\GenesisBlock-ki17'
npm ci --ignore-scripts
npm run build:debug             # napi build --platform
```

The native addon must be built from the pinned GenesisBlock checkout before
running the worker. The worker needs Python `3.12.10` x64 and the exact
sidecar dependencies in [`requirements.txt`](requirements.txt):

```powershell
py -3.12 -m venv .venv
& .\.venv\Scripts\python.exe -m pip install -r genesisrag17-worker\requirements.txt
& .\.venv\Scripts\python.exe --version       # Python 3.12.10
& .\.venv\Scripts\python.exe -c "import numpy, onnxruntime, tokenizers; print(numpy.__version__, onnxruntime.__version__, tokenizers.__version__)"
```

The model is `intfloat/multilingual-e5-small`, revision
`614241f622f53c4eeff9890bdc4f31cfecc418b3`. Acquire a complete snapshot from
the pinned revision, then verify every file below. The worker repeats these
size and SHA-256 checks at startup and fails closed; it never substitutes a
different revision or a fallback embedder. The `upstream OID` column is the
file/tree or LFS object identifier recorded from that pinned Hugging Face
revision.

| file | bytes | local SHA-256 | upstream OID |
| --- | ---: | --- | --- |
| `onnx/model.onnx` | 470268510 | `ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665` | `f9c7ec44162ecb2ae1340185b24a43d83d606a00` |
| `tokenizer.json` | 17082730 | `0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39` | `c9b2fea3119ca8886380e5f47bffc5ea7a6e0ffa` |
| `tokenizer_config.json` | 443 | `a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b` | `059214673d9d6d2ee319411e2ffec8c024b816d5` |
| `special_tokens_map.json` | 167 | `d05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7` | `e0b1d18f602b9acb69e1940d4af2d7ba09a2d626` |
| `config.json` | 655 | `69137736cab8b8903a07fe8afaafdda25aac55415a12a55d1bffa9f581abf959` | `60a2a84020f1d74cc53ea9e8c4e91cf4af6c2b68` |

The worker's `MODEL_ARTIFACTS` table in `src/worker.mjs` is the executable
manifest. A runtime cache is acceptable only after it matches this manifest.
For a fresh download, use the Hugging Face revision URL with a local target,
then point `GENESIS_WORKER_MODEL_DIR` at the resulting snapshot directory:

```text
https://huggingface.co/intfloat/multilingual-e5-small/tree/614241f622f53c4eeff9890bdc4f31cfecc418b3
```

## Isolated worker run

The following example uses only test paths, test credentials and the synthetic
golden fixture. Replace `GENESIS_WORKER_MSP_COMMAND` with the local MSP stdio
bridge used by the integration harness; do not put a production credential in
this file or in a committed environment file.

```powershell
$env:GENESIS_WORKER_DB_PATH = 'C:\Users\pc\workspace\ki17-runtime\stores\genesisrag17-test'
$env:GENESIS_WORKER_SCOPE = '{"portfolioId":"portfolio-test","tenantId":"tenant-test","businessId":"business-test","workspaceId":"","agentId":"","visibility":"private"}'
$env:GENESIS_WORKER_CREDENTIAL = 'worker-credential-test'
$env:GENESIS_WORKER_QUERY_TOKEN = 'query-token-test'
$env:GENESIS_WORKER_MODEL_DIR = 'C:\Users\pc\.cache\huggingface\hub\models--intfloat--multilingual-e5-small\snapshots\614241f622f53c4eeff9890bdc4f31cfecc418b3'
$env:GENESIS_WORKER_BENCHMARK_FIXTURE = 'C:\Users\pc\workspace\zuri-ai-ki17\apps\server\tests\fixtures\genesisrag17-corpus-v1.json'
$env:GENESIS_WORKER_MSP_COMMAND = 'node'
$env:GENESIS_WORKER_MSP_ARGS = '["apps/msp-server/bin/msp-server.mjs"]'
$env:GENESIS_WORKER_MSP_CWD = 'C:\Users\pc\workspace\msp-ki17'
$env:GENESIS_WORKER_MSP_TIMEOUT_MS = '30000'
$env:GENESIS_WORKER_PORT = '0'

Set-Location 'C:\Users\pc\workspace\GenesisBlock-ki17'
npm test --prefix genesisrag17-worker
node genesisrag17-worker\src\cli.mjs
```

`GENESIS_WORKER_SCOPE` is deliberately an exact six-field private scope.
`GENESIS_WORKER_DB_PATH` must be a new isolated TEST directory for the run.
The worker creates `genesisrag17/state.json`, durable decision and stage-failure
records, graph/write/publication outboxes, retained snapshots and the worker
owned `lexical.sqlite` sidecar below that directory.

The CLI prints the bound loopback endpoint as one JSON line. Query with the
bearer token and a scope-matching `genesisrag17.v1` request:

```powershell
$body = '{"schemaVersion":"genesisrag17.v1","scope":{"portfolioId":"portfolio-test","tenantId":"tenant-test","businessId":"business-test","workspaceId":"","agentId":"","visibility":"private"},"query":"Which company employs Alice?","topK":5}'
Invoke-RestMethod -Method Post -Uri 'http://127.0.0.1:<port>/query' -Headers @{ Authorization = 'Bearer query-token-test' } -ContentType 'application/json' -Body $body
```

## Honest lane and recovery behavior

- **Vector** uses the native HNSW index, 384-dimensional CPU embeddings and a
  separate collection for each scope/generation. This prevents a global
  collection top-k followed by generation filtering from losing historical
  hits.
- **Lexical** uses worker-owned SQLite FTS5 because the current native binding
  has no usable lexical query method. It is reported as
  `implementation: worker_sqlite_fts5` in the worker implementation and is
  scoped by tenant and generation.
- **Graph** and **SQLite** are checked through native commit, flush, save-state
  and SQL readback. Graph counts include source/parsed/chunk/entity/fact/held
  objects and their lineage, mention, assertion and evidence edges.
- **Bitemporal** is `ready` only after an actual native Query IR temporal
  readback for mapped valid-time assertions. Facts with GKS's explicit
  `validFrom: "not_applicable"` and `validTo: "not_applicable"` are reported
  `not_applicable`; transaction-time stamps alone do not become a fabricated
  valid-time mapping. Derived summaries are outside the temporal assertion
  set by contract.
- **Provenance** verifies source/chunk ids, UTF-16 offsets, UTF-8 hashes and
  citation payloads against persisted native rows. A benchmark citation check
  covers every returned row, including non-relevant rows.

Writes follow graph-only stage 13, graph receipt, stage 14 enrichment, stage 15
embedding, stage 16 final commit/readback and the GKS quality gate. A denied
embedding policy produces an actual persisted Stage 15 failure with six
metrics; it never reports a zero-vector success. Transport loss leaves a
durable outbox for retry. Native commits use deterministic transaction ids and
frontier checks so a lost receipt does not blindly recommit.

The worker accepts fault-injection hooks at native-commit-before-receipt,
before-pointer-replacement and after-pointer-replacement-before-publication-
outbox. Recovery reuses the exact candidate snapshot, pointer and publication
receipt data. A prepared snapshot cannot be queried until its id appears in
the atomically replaced pointer's retained published-history list.

## Current native limitations

The N-API binding at this pinned commit does not expose a `close` method. The
worker closes its HTTP server, Python sidecar, FTS5 database and lock, but a
true native handle restart must be performed by a dedicated child process.
The worker therefore enforces one store owner with a PID/token lock and never
deletes a live owner's lock. There is no OS scheduler; `start`, `stop` and
`resume` drive the polling loop explicitly. Query `topK` is bounded to 1..100
at the worker endpoint, while the pipeline acceptance fixture uses top-k 5.

Run the focused package proof with:

```powershell
Set-Location 'C:\Users\pc\workspace\GenesisBlock-ki17'
npm test --prefix genesisrag17-worker
```

The tests require the pinned model artifacts and the built native addon. They
exercise native graph/vector writes and readback, real CPU embedding, FTS5
fusion, scope isolation, policy Stage 15 failure, pointer crash recovery and
prepared-snapshot visibility.
