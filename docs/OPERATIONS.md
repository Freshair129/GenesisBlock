---
status: current
---

# GenesisBlockDB — Operations Runbook

Operational guidance for running the REST server (`genesis-db-server`) or an
embedded (NAPI) instance in production. Pairs with `SECURITY.md` (vulnerability
policy) and `CHANGELOG.md`.

## On-disk layout

Everything lives under the database path (`OpenOptions.path`):

| File / pattern        | Contents                                                        |
| --------------------- | --------------------------------------------------------------- |
| `genesis-graph.wal`   | Write-ahead log (append-mostly; compacted into the live state). |
| `state.json`          | Snapshot manifest: nodes, edges, collection manifest, clocks.   |
| `vec_<name>.bin`      | Per-collection vector arena (one per collection).               |
| `meta_<name>.bin`     | Per-collection vector metadata.                                 |
| `fvec_<name>.bin`     | Exact-f32 rerank sidecar (only for quantized rerank collections).|
| `identity.bin`        | ed25519 swarm signing key. **Secret — back up securely.**       |

## Backup

The database is durable via the WAL, but a consistent backup should capture a
quiesced snapshot:

1. Trigger a snapshot so the WAL is compacted into `state.json` (the embedded
   API exposes `save_state()`; the WAL is also replayed on next open).
2. Copy the **entire database directory** (all files above). The set is
   self-consistent only when copied together — prefer a filesystem snapshot
   (LVM/ZFS/VSS) or copy while the process is stopped.
3. Store `identity.bin` with the same protection as any private key. Losing it
   changes the node's swarm identity; leaking it lets an attacker sign events as
   this node.

## Restore

1. Stop the server.
2. Restore the full directory to the database path.
3. Start the server. On open, the engine replays `genesis-graph.wal` on top of
   `state.json` and rebuilds every collection's HNSW index
   (`rehydrate_hnsw_index`). First open after a large restore may take time
   while indexes rebuild — vectors are searchable once rehydrate completes.

## Health checks

- `GET /v1/status` — engine liveness; returns `503` while the index is
  rebuilding (`is_rebuilding`). Use this as the readiness probe so traffic is
  held off during rebuilds.
- `GET /v1/swarm/status` — peer id and swarm state.
- Embedded: `index_lag()` reports vectors staged but not yet inserted into HNSW
  (async indexing backlog). Non-zero means recent writes are not yet searchable;
  `flush_index()` drains it (read-your-write).

## Networking note (swarm gossip)

The gossip/discovery layer binds **UDP `0.0.0.0:30001`** and broadcasts to
`255.255.255.255:30001`. Two consequences for production:

- Only one gossip-enabled instance per host can hold the fixed discovery port;
  the bind now fails gracefully (logged, task exits) rather than panicking.
- On a shared LAN, multiple GenesisBlock instances will discover and CRDT-sync
  with each other over this port. Isolate instances at the network layer
  (separate subnet / firewall UDP 30001) unless cross-instance sync is intended.

## Rollback

This engine is append-mostly and bitemporal — prefer rolling *forward*:

- **Bad data write:** nodes evolve by `supersede_node` and edges soft-delete via
  `retract_edge` (bitemporal). Time-travel (`as_of`) still sees prior state; you
  rarely need a file-level rollback for a data mistake.
- **Bad deploy / corruption:** stop the server, restore the last good directory
  backup (see Restore), start. Because the WAL is append-only, a backup taken
  before the bad window plus replay recovers to a consistent point.

## Rollback / alert triggers

- Readiness (`/v1/status`) returns `503` for longer than an index rebuild should
  take.
- Error rate on `/v1/*` rises after a deploy.
- `index_lag()` grows without bound (indexing thread stalled).
- Disk usage on the database path approaches capacity (WAL + arenas).
