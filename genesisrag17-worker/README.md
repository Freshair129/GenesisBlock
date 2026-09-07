# GenesisRAG17 Tier 4 worker

This package is the isolated **TEST** worker for stages 13, 15 and 16 of the
GenesisRAG17 pipeline. It owns one GenesisBlock native store process, receives
graph decisions and receipts through the MSP tool boundary, and exposes an
authenticated loopback query endpoint. It never opens the GKS store directly,
does not read the Edge store, and has no LLM-assisted extraction path.

The wire contract is the [GenesisRAG17 implementation
contract](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/plans/GENESISRAG17-CONTRACT.md)
(`genesisrag17.v1`, currently `1.2.1b`). The [17-stage source
specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md),
[17-stage execution
flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md)
and the local [separate-worker/publication ADR](../docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md)
define the boundary. The worker persists source, parsed artifact, chunk,
mention, entity, fact, held-fact, derived-object and citation lineage in the
native graph/SQLite projections before it reports a receipt. Candidate
snapshots are prepared on disk but are queryable only after the atomic
published pointer names their snapshot id. Previous published snapshot ids
remain queryable for correction and historical tests. The [pinned historical
acceptance report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md)
records the isolated synthetic evidence and its non-production limits.

## Reproducible TEST setup

Use an x64 Windows Node.js `24.18.x` runtime for this package. The integration
harness supplies an isolated runtime path; set `$Ki17Runtime` to that path
before running the commands below. This repository does not bundle or imply
ownership of that runtime:

```powershell
$GenesisRoot = 'C:\Users\pc\workspace\GenesisBlock-ki17'
$Ki17Runtime = 'C:\Users\pc\workspace\ki17-runtime'
$env:Path = "$Ki17Runtime;$env:Path"
node --version                 # v24.18.0

Set-Location $GenesisRoot
npm ci --ignore-scripts
npm run build:debug             # napi build --platform
```

The native addon must be built from the pinned GenesisBlock checkout before
running the worker. The worker needs Python `3.12.10` x64 and the exact
sidecar dependencies in [`requirements.txt`](requirements.txt):

```powershell
Set-Location $GenesisRoot
py -3.12 -m venv .venv
& .\.venv\Scripts\python.exe -m pip install -r genesisrag17-worker\requirements.txt
$env:GENESISRAG17_PYTHON = (Resolve-Path '.venv\Scripts\python.exe').Path
& $env:GENESISRAG17_PYTHON --version       # Python 3.12.10
& $env:GENESISRAG17_PYTHON -c "import numpy, onnxruntime, tokenizers; print(numpy.__version__, onnxruntime.__version__, tokenizers.__version__)"
```

`GENESISRAG17_PYTHON` is the explicit Python executable passed to the worker's
embedding sidecar. When it is set, the worker uses that executable and fails
closed on a missing or invalid environment; it does not silently select a
different interpreter.

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

The following example uses only isolated TEST paths, synthetic credentials and
the golden fixture. Replace the path variables and
`GENESIS_WORKER_MSP_COMMAND` with the local MSP stdio bridge used by the
integration harness; do not put a production credential in this file or in a
committed environment file. `$ZuriRoot`, `$MspRoot` and `$GksRoot` are explicit
harness parameters. The worker uses `$MspRoot` only as the working directory of
the MSP relay and never opens `$GksRoot` directly.

```powershell
$GenesisRoot = 'C:\Users\pc\workspace\GenesisBlock-ki17'
$ZuriRoot = 'C:\Users\pc\workspace\zuri-ai-ki17'
$MspRoot = 'C:\Users\pc\workspace\msp-ki17'
$GksRoot = 'C:\Users\pc\workspace\gks-ki17'       # harness parameter; never opened by worker
$Ki17Runtime = 'C:\Users\pc\workspace\ki17-runtime'
$ModelDir = 'C:\Users\pc\.cache\huggingface\hub\models--intfloat--multilingual-e5-small\snapshots\614241f622f53c4eeff9890bdc4f31cfecc418b3'
$Fixture = Join-Path $ZuriRoot 'apps\server\tests\fixtures\genesisrag17-corpus-v1.json'
$env:GENESIS_WORKER_DB_PATH = Join-Path $Ki17Runtime 'stores\genesisrag17-test'
$env:GENESIS_WORKER_SCOPE = '{"portfolioId":"portfolio-test","tenantId":"tenant-test","businessId":"business-test","workspaceId":"","agentId":"","visibility":"private"}'
$env:GENESIS_WORKER_CREDENTIAL = 'worker-credential-test'
$env:GENESIS_WORKER_QUERY_TOKEN = 'query-token-test'
$env:GENESIS_WORKER_MODEL_DIR = $ModelDir
$env:GENESIS_WORKER_BENCHMARK_FIXTURE = $Fixture
$env:GENESIS_WORKER_MSP_COMMAND = 'node'
$env:GENESIS_WORKER_MSP_ARGS = '["apps/msp-server/bin/msp-server.mjs"]'
$env:GENESIS_WORKER_MSP_CWD = $MspRoot
$env:GENESIS_WORKER_MSP_TIMEOUT_MS = '30000'
$env:GENESIS_WORKER_PORT = '0'
$env:GENESISRAG17_PYTHON = (Resolve-Path (Join-Path $GenesisRoot '.venv\Scripts\python.exe')).Path

Set-Location $GenesisRoot
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

The source and worker grants are different MSP principals. The source process
uses a `role: source` entry in `MSP_PIPELINE_PRINCIPALS` for submit, evidence
pull and query. The worker's `GENESIS_WORKER_CREDENTIAL` uses a separate
`role: worker` entry for claim, graph receipt, write receipt, gate, publication
receipt, stage failure and query. MSP removes caller-supplied identity and
relays with `MSP_GKS_PIPELINE_CREDENTIAL`; GKS checks that value against
`GKS_PIPELINE_RELAY_CREDENTIAL`. `GENESIS_WORKER_QUERY_TOKEN` (also known to
the relay as `MSP_PIPELINE_WORKER_TOKEN`) authenticates only the worker's
loopback HTTP endpoint. Configure these grants in the isolated MSP/GKS process
environment with explicit test values; never reuse the source credential as a
worker credential or commit a real value.

## Ordered physical flow and publication

The worker implements the physical sequence required by the contract:

1. GKS returns an immutable decision from Stages 9–12 through the MSP relay.
   The decision initially has no derived summaries.
2. The worker performs a graph-only native transaction for Stage 13, including
   source, parsed, chunk, mention, entity, verified-fact and held rows. It
   writes no vectors or derived summaries, then flushes, checkpoints and reads
   back the native graph/SQLite state.
3. The worker calls `msp_pipeline_graph_receipt`. MSP relays the receipt to
   GKS, which closes Stage 13 once, runs `enrich_v1` Stage 14 and returns a
   separate immutable `derived` result and `derivedHash`.
4. The worker embeds the allowed chunks with the pinned CPU model for Stage 15,
   commits derived rows and vectors for the candidate generation, and performs
   Stage 16 flush/checkpoint, six-lane readback and the independent fixture
   benchmark. It then sends the final write receipt.
5. GKS verifies both physical receipts and evaluates the five Stage 17 quality
   dimensions. A quality/policy failure creates actual failure evidence and no
   publication.
6. For an allowed verdict, the worker writes a prepared snapshot, atomically
   replaces the pointer and retained published-history list, and sends
   `msp_pipeline_publication_receipt`. Only an accepted publication receipt
   allows GKS to emit successful Stage 17 evidence and Tier 1 to finish.

The candidate file is never queryable merely because it exists. A query reads
the authoritative pointer once and accepts a requested historical snapshot
only when its id belongs to `publishedSnapshotIds`; all results bind to one
generation. Old published generations and their citations remain available for
correction. Graph, write, stage-failure and publication requests use durable
outboxes and replay the exact payload after reply loss or process restart.

## Honest lane and recovery behavior

Stage 16 reports all six lanes with an actual numeric `objects` count:

| Lane | Status in the current TEST profile | Evidence / implementation |
|---|---|---|
| Vector | `ready` | Native HNSW write, flush, per-scope/generation readback; CPU E5 artifacts verified |
| Lexical | `ready` | Worker-owned FTS5 sidecar; manifest implementation is `worker_sqlite_fts5` |
| Graph | `ready` | Native commit, flush/checkpoint and graph readback |
| SQLite | `ready` | Engine-owned internal projection SQL readback; callers never open it |
| Bitemporal | `ready` only for actual native valid-time Query IR readback; `not_applicable` for explicit no-valid-time fixture data | Transaction time alone is not temporal-index evidence |
| Provenance | `ready` | Persisted source/chunk/citation IDs, UTF-8 hashes and UTF-16 offsets resolve after restart |

An unsupported required lane fails the GKS gate. `not_applicable` is an
explicit input applicability result, not a hidden fallback.

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
$GenesisRoot = 'C:\Users\pc\workspace\GenesisBlock-ki17'
Set-Location $GenesisRoot
npm test --prefix genesisrag17-worker
```

The tests require the pinned model artifacts and the built native addon. They
exercise native graph/vector writes and readback, real CPU embedding, FTS5
fusion, scope isolation, policy Stage 15 failure, pointer crash recovery and
prepared-snapshot visibility.

For the cross-repository ownership, extension points and recovery matrix, see
[FLOW--GENESISRAG17-PIPELINE.md](../docs/FLOW--GENESISRAG17-PIPELINE.md),
[GENESISRAG17-EXTENSION-MAP.md](../docs/GENESISRAG17-EXTENSION-MAP.md) and
[ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md](../docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md).
