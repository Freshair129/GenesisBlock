---
doc_id: RCA--STANDALONE-SERVER-RESTART-RECOVERY-2026-10-15
version: "2026-10-15"
created_at: "2026-10-15T00:00:00+07:00,Codex"
last_update: "2026-10-15T00:00:00+07:00,Codex"
status: evidence
owner: GenesisBlockDB Engineering
attributes:
  domain: standalone-server-recovery
  scope: issue-222
  risk: MEDIUM
  complexity: C-2
---

# RCA: standalone server metadata after restart

Date: 2026-10-15 (Asia/Bangkok)
Issue: [#222](https://github.com/Freshair129/GenesisBlock/issues/222)

## Symptom

The published Windows server archive for v0.2.9 acknowledges node writes and
returns each node's own properties during vector search. After terminating the
server process and reopening the same database, vector search still returns the
expected node ID, but properties rotate to the next node. An independent
Python HTTP harness reproduced the mismatch in 3/3 fresh databases. The edge
ID, endpoints, relation and properties survived the same restart.

## Evidence

- Published artifact: `genesisblockdb-server-v0.2.9-x86_64-pc-windows-msvc.zip`,
  tag commit `8091a56f2592b8cb28bbae12421cacc9352c2cab`, SHA-256
  `5d54b59ee0f940616ea0422618bb783bc45a1bedebb3c242641bacecfb134c34`.
- The release identifies engine version `0.2.9` and storage schema `4`.
- Current main under audit is `31f8a9279e4360fef115f462c7244fbc01583840`.
  Its storage default is schema 6. `Storage::add_node` now records local graph
  mutations through `persist_local_graph_mutations`; the v0.2.9 source snapshot
  used `persist(Event::Node(...))`.
- The current-main standalone REST debug executable passed the same default
  retention, named-collection, four-node Thai-property/vector, edge, auth and
  forced-process-restart assertions in 3/3 fresh fixtures. Its Windows debug
  executable SHA-256 was
  `b8a5ee83b06bd57917bb4decfa58fa123ed2355d0bf148446ae6d7dcc3a73e0b`.
- The permanent `server_restart_recovery_tests` integration target passed both
  test functions (2/2); the REST process test itself repeats three fresh
  default-retention databases, and the other test verifies graceful Storage
  close/reopen. It checks exact IDs/properties for four Thai-property nodes,
  orthogonal vectors in a named collection, edge identity/endpoints/relation/
  properties, and unauthenticated status rejection. The standalone server
  executable used by the tracked debug test had SHA-256
  `EEEC2186CBBAD089F960DE8774CA7125076CAA9ED3E9F8394903992C00B8DE7B`.
  This is separate from the external REST harness executable above.
  Command: with `CARGO_BUILD_JOBS=2` and
  `CARGO_TARGET_DIR=G:\AI-Studio\verification\2026-10-15-readiness-fixes\GenesisBlock\target`,
  `cargo test --features bins --test server_restart_recovery_tests -- --nocapture`.
  Full output is in the local `genesis-restart-test-debug.log` evidence file.
- The current-main direct-core harness also passed 3/3 with default retention.
- The checked-in server-distribution workflow now gates packaging on the same
  process-level regression under the release profile. The release-profile
  integration test and produced archive were not completed in this local run;
  debug test evidence does not qualify a release artifact.

The release and current-main implementations differ, but no bisect or isolated
causal experiment establishes which individual source change prevents the
property rotation. This RCA therefore does not claim a proven single-line
engine root cause.

## Root cause

The affected shipped artifact predates the current schema-6 transaction-backed
graph mutation path. The server distribution workflow packaged a standalone
binary without a process-level REST restart/recovery check that asserted
node-property ownership after recovery. Unit and storage fixtures did not
exercise that shipped boundary. The precise internal trigger in the v0.2.9
artifact remains unisolated.

## Prevention

1. Keep the forced-process-restart fixture as a Rust integration test that
   starts the actual `genesis-db-server` executable, writes through REST, kills
   only its child server process after successful acknowledgments, and reopens
   the same isolated fixture.
2. Run that integration test in the standalone server distribution workflow
   using the release profile before packaging the same target binary.
3. Assert exact node IDs and properties through vector search and graph reads;
   assert edge IDs, endpoints, relations and properties across restart.
4. Keep artifact version, source revision, SHA-256, retention configuration,
   and test outcome together in readiness evidence. Do not replace the
   historical v0.2.9 release artifact in place.
