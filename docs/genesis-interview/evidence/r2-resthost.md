# Self-hosted (Docker/REST) readiness — code-grounded audit

## 1. What the REST surface already ships

The standalone server is real and closer to "deployable" than a stub — it already has env-var config, bearer-token auth, CORS policy, body limits, and Prometheus metrics.

**Server bootstrap (`src/main.rs`)**
- Env-var config: `GENESIS_DATA_DIR` (default `.brain/gks/storage`, src/main.rs:19-20), `GENESIS_PORT` (default 3000, src/main.rs:21-24), `GENESIS_HOST` (default 127.0.0.1 — secure-by-default localhost bind; log tells you to set `0.0.0.0`, src/main.rs:25-28, 52).
- Optional API key: `GENESIS_API_KEY` env var; loud startup warning when unset (src/main.rs:37-42).
- Built behind `--features bins`; on Linux use `--no-default-features --features bins` (CLAUDE.md core/napi split; docs/DEPLOY-CHECKLIST--V0.2.0.md:45).

**Routes (`src/router.rs:619-679`, all under one guarded router):** `/v1/bulk/nodes|edges|rebuild`, `/v1/query/hql`, `/v1/node/add|supersede`, `/v1/edge/add|retract`, `/v1/collection/create`, `/v1/collections`, `/v1/vector/add`, `/v1/insight/drift/:id|communities|gaps|rebuild`, `/v1/query`, `/v1/search/hybrid`, `/v1/reason/context`, `/v1/status`, `/v1/version`, `/v1/swarm/status`, `/v1/consensus/propose|vote|sign-vote|verify` — plus root `GET /metrics` deliberately outside the auth guard (src/router.rs:663-673, tested at tests/rest_api_tests.rs:904).

**Ops surface already present**
- Auth: `api_key_guard` middleware requires `Authorization: Bearer <key>` on every /v1 route when the key is set (src/router.rs:684-702), with 401 tests (tests/rest_api_tests.rs:949-987). Single static key only.
- Prometheus `/metrics`: hand-rolled text exposition — nodes/edges/collections totals, `index_lag`, memory estimate, `is_rebuilding`, per-collection vector counts and sidecar/arena resident/disk bytes (src/router.rs:492-594).
- `/v1/status` (ExtendedStatus: open/read_only/node_count/edge_count/memory/index_lag/per-collection info, src/router.rs:98-115, 428-453) and `/v1/version` (engine+schema version for upgrade tooling, src/router.rs:458-464).
- Body limits: 64 MB global cap (src/router.rs:678), 256 KiB on `/v1/query/hql` (src/router.rs:627-630).
- CORS: default localhost-only allowlist; `GENESIS_CORS_ORIGIN=*` or exact origin override (src/router.rs:709-728).
- Availability guard: query handlers return 503 while `is_rebuilding` (src/router.rs:351-357).
- Body-shape tolerance: `/v1/query/hql` accepts raw string or `{"query": ...}` via untagged `HqlBody` (src/router.rs:329-343).

## 2. What is ABSENT for production self-host

| Item | Evidence | Status |
|---|---|---|
| Container image | Zero Dockerfile/compose/.dockerignore in repo (only glob hit is inside `react-native-genesisdb/node_modules/recast`); no docker/container step in any of the 9 `.github/workflows/*` files | **Absent entirely** |
| TLS | No rustls/native-tls/openssl anywhere in Cargo.toml; plain `TcpListener` + `axum::serve` (src/main.rs:51-53) | Absent — reverse-proxy only |
| AuthZ / multi-tenancy | One global `Arc<RwLock<Storage>>` in `AppState` (src/router.rs:26); one shared API key = all-or-nothing admin; collections have zero access control; governance tiers exist in engine but no per-caller identity over REST | Absent |
| Graceful shutdown | `axum::serve(listener, app).await` with no `with_graceful_shutdown` / signal handling (src/main.rs:53); no explicit `save_state()`/WAL checkpoint on exit — relies on WAL replay at next open | Absent |
| Rate limiting | No governor/tower rate layer; tower-http features are only `cors,trace,limit` (Cargo.toml:72) | Absent |
| Online backup/restore | `save_state()` exists in core (src/lib.rs:4537, async wrapper 5853, does WAL compaction per project_wal_compaction memory) but there is NO REST route to trigger snapshot/checkpoint, and no restore/export endpoint at all | Absent over REST |
| Health endpoint | No `/health` or `/livez`; `/v1/status` is behind the API-key guard (src/router.rs:648, guard covers all /v1), so an orchestrator health probe needs the secret; unauthenticated `/metrics` is the only workaround | Gap |
| Upgrade/migration | `/v1/version` reports `schema_version` (src/router.rs:458-464) and legacy snapshots migrate on open (multi-collection + u64→u128 edge keys per CLAUDE.md), but no documented rolling-upgrade or downgrade story; DEPLOY-CHECKLIST--V0.2.0.md is an npm-publish checklist, not a server-ops doc | Partial |
| Concurrent writers | Single-process only: `AppState.storage` is `Arc<RwLock<Storage>>`; every mutating handler takes `.write()` (e.g. src/router.rs:172, 202), serializing all writes through one process-wide lock ahead of the internal WAL writer thread. No file-lock against a second process opening the same dir was found (UNVERIFIED — did not exhaustively read the 6k-line open path) | Single-writer by construction |
| Replication/HA | CRDT primitives exist in core — `reconcile_state(Vec<SignedEvent>)` (src/lib.rs:4115), `events_since(from_clock)` (src/lib.rs:5393), `get_merkle_root` (src/lib.rs:5314), tested in tests/crdt_sync_tests.rs — but REST exposes only read-only `/v1/swarm/status` + consensus voting (src/router.rs:650-654). There is **no `/v1/sync/events` or `/v1/sync/reconcile` route**: two REST servers cannot actually sync over HTTP today; no peer-transport, no anti-entropy loop | Core-only, no wire surface |

## 3. Effort estimates (grounded in what exists)

- **Dockerfile + compose + CI image build — S.** Static musl/glibc build of `genesis-db-server` (`--no-default-features --features bins` already links cleanly on Linux per the core/napi split), volume-mount `GENESIS_DATA_DIR`, expose 3000. All config is already env-var driven (src/main.rs:19-28,37) so the container needs no new code. This is the genuinely "near-free" part.
- **Graceful shutdown — S.** Add `with_graceful_shutdown` + SIGTERM handler calling `save_state()` (already exists, src/lib.rs:4537). ~30 lines.
- **Unauthenticated `/health` — S.** Same merge-outside-guard pattern `/metrics` already uses (src/router.rs:671-673).
- **Backup/snapshot REST endpoint — S/M.** S to expose `save_state()` as `POST /v1/admin/checkpoint` (WAL compaction is done and tested); M if you want a consistent tarball/export-download and a restore path.
- **Rate limiting — S/M.** tower-based per-IP layer is S; per-key quotas need the multi-key work first.
- **TLS — S (punt) / M (native).** Document "terminate at Caddy/nginx" in the compose file = S; native rustls listener = M.
- **Real authn/authz (multiple keys, roles, per-collection scope) — M/L.** Guard middleware and tests exist as the seed (src/router.rs:684-702), but key storage, roles mapped onto the existing governance tiers, and per-collection ACL are new design.
- **Multi-tenancy — L.** Everything assumes one global `Storage` (one WAL, one id-intern table, one swarm identity); per-tenant isolation means multiple Storage instances or namespace plumbing through the 6k-line core.
- **Replication/HA over REST — L.** The CRDT/Merkle core is real and tested, but the transport (sync routes, peer discovery/registration, anti-entropy scheduler, conflict observability, failover semantics) is all missing; treat current swarm sync as embedded/NAPI-maturity, not server-maturity.

**Bottom line:** a *single-node, single-tenant* Docker self-host is genuinely near-free — roughly a Dockerfile plus three S items (shutdown, /health, checkpoint route), because config, auth gate, metrics, CORS, and body limits already shipped. What is NOT near-free is anything implied by "production database server": TLS-native, multi-key authz, tenancy, online backup/restore UX, and HA/replication — the last two are M–L and L respectively.