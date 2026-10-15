---
doc_id: TDD--STANDALONE-SERVER-RESTART-RECOVERY
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

# TDD — Standalone server restart recovery

## 1. Design decision

The process regression is an integration test gated by Cargo feature `bins`.
Cargo supplies the `genesis-db-server` executable path to the test. The test
starts that executable with an isolated temporary data directory, loopback
host, reserved local port, API key and default retention. It writes through
HTTP, kills only that child after the REST acknowledgments, starts a new server
against the same directory, and compares exact REST results.

The process test repeats the forced restart against three fresh default-
retention databases. Each fixture uses four basis vectors and unique Thai
property values so an ID to property rotation is observable without a model
download or model inference. Every fixture covers status authentication,
vector search and an edge query. A separate Storage fixture opens, writes,
drops the handle normally and reopens to check graceful persistence through
the supported library boundary.

```mermaid
sequenceDiagram
    participant T as Integration test
    participant S1 as Server child 1
    participant D as Temporary database
    participant S2 as Server child 2
    T->>S1: Start with loopback, API key, default retention
    T->>S1: POST collection, four nodes, one edge
    S1->>D: Persist acknowledged writes
    T->>S1: Query vector and edge; assert exact IDs and properties
    T-xS1: Force-stop only child after acknowledgments
    T->>S2: Restart with the same directory
    S2->>D: Recover journal and projection
    T->>S2: Query vector and edge
    T->>T: Assert ID, properties, endpoints, relation and edge properties
```

The CI test runs all three process fixtures in one integration-test invocation.
The graceful Storage close/reopen assertion is a separate test and does not
stand in for the process-level REST recovery check.

## 2. Release workflow

The server-distribution target job builds the release server for its native
runner/target pair. It then runs the required-feature integration test in the
release profile on that same runner and target. Cargo's test environment points
to the release-profile server executable. Packaging follows only after the
process test succeeds, so the tested executable is the one placed in the
archive.

## 3. Failure handling

Any non-2xx write, readiness timeout, unexpected child exit, malformed response,
wrong vector result, property mismatch, edge mismatch or failed authentication
assertion fails the test. The test's cleanup kills only its owned child and
removes the temporary fixture through `TempDir` cleanup. It never reads or
opens a user database directory.

## 4. Compatibility and non-goals

No REST route, JSON shape, persistence format or retention default changes.
The change adds recovery coverage to the standalone server release gate. It
does not prove semantic embedding quality or release-registry availability.
