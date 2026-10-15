---
doc_id: SRD--STANDALONE-SERVER-RESTART-RECOVERY
owner: GenesisBlockDB Engineering
version: "0.1.0b"
created_at: "2026-10-15T00:00:00+07:00,Codex"
last_update: "2026-10-15T00:00:00+07:00,Codex"
status: beta
attributes:
  domain: standalone-server-recovery
  scope: issue-222
  complexity: C-2
  risk: MEDIUM
---

# SRD — Standalone server restart recovery

## 1. Purpose and boundary

This requirement closes a reproduced recovery gap in the published v0.2.9
Windows standalone server. It covers only the existing generic REST server and
Genesis persistence contract. It does not add an asset schema, query feature,
workflow dispatch, or user-database migration.

## 2. Requirements

### SR-1: Acknowledged graph data survives forced process restart

For a server built from the current source, after a successful REST response
for a node write, restarting the server process against the same database
preserves the node's exact ID and JSON properties. If the node has an embedding,
vector search in the same named collection returns that node with those exact
properties.

### SR-2: Edges survive process restart without identity or metadata drift

After a successful REST response for an edge write, restart preserves its ID,
source, destination, relation and JSON properties.

### SR-3: Graceful close and reopen preserve the same data

Closing a `Storage` handle through its normal close/drop path and reopening the
same fixture preserves node properties, vector ownership and edge fields.
This is a distinct check from forced process termination.

### SR-4: Server authentication remains active across the fixture

When `GENESIS_API_KEY` is set, the standalone server rejects a status request
without a bearer token and accepts the same request with the configured key.

### SR-5: Release artifacts are tested before packaging

The server distribution workflow runs the process-level recovery regression
under the release profile for each target before packaging that same target's
server binary. The archive and checksum remain versioned outputs; a historical
release is never replaced as a bug fix.

## 3. Acceptance

- Three fresh default-retention databases pass the forced-restart REST fixture.
- Each fixture writes four nodes with distinct Thai JSON properties and
  orthogonal four-dimensional vectors, then verifies all four after restart.
- Edge identity, endpoints, relation and properties match exactly before and
  after restart.
- Authentication checks pass.
- A separate graceful `Storage` close/reopen fixture preserves node, vector and
  edge data.
- The process test runs with the compiled server executable, not the in-process
  router.
- The test only writes under its test-owned temporary directory and only stops
  the child process it started.

## 4. Evidence limits

The deterministic vectors test storage ownership and recovery, not embedding
quality or semantic ranking. This fixture does not qualify long-term durability,
concurrent writes, performance, power-loss behavior, or all operating systems.
The v0.2.9 release failure remains part of historical evidence until a new
versioned artifact passes the release workflow and consumer smoke.
