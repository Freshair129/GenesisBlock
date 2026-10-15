# RCA — node properties shift after forced REST server restart

**Status:** confirmed on the historical v0.2.9 Windows artifact; current-main
recovery invariant verified by standalone REST process regression · **Issue:**
[#222](https://github.com/Freshair129/GenesisBlock/issues/222) · **Risk:**
MEDIUM / C-2

## Symptom

After the standalone server acknowledged node writes and was force-terminated,
the historical v0.2.9 server returned the next node's properties for each of
the first three vector-hit node IDs. Vector IDs remained present, so creative
provenance properties could be associated with the wrong asset.

## Evidence

- The v0.2.9 Windows release archive was tested through REST with four uniquely
  tagged nodes and orthogonal 1536-dimensional embeddings in three fresh
  default-retention databases. All 3/3 forced-restart runs reproduced the
  property shift.
- Historical release source commit: `8091a56f2592b8cb28bbae12421cacc9352c2cab`.
- Historical release archive SHA-256:
  `5D54B59EE0F940616EA0422618BB783BC45A1BEDEBB3C242641BACECFB134C34`.
- Historical server executable SHA-256:
  `7EBD3F71C4EE92FD35FF7BFE11822A2CF9C16B4C306E6033B5135E70CEEE6878`.
- The current-main source correction is present in `4f02d6b`: loading
  `nodes.bin` now initializes the next internal node key at zero and advances
  from saved keys with `max(k.saturating_add(1))`.
- The regression `scripts/test-rest-restart-recovery.py` exercises the built
  standalone server over REST in three fresh default-retention databases. Each
  fixture checks properties and vector hits after a hard kill, then after a
  graceful checkpoint and reopen. The same test is wired into the Linux,
  Windows, and macOS server-binary build matrix.
- Tested current-main server source commit: `637ee47d33f056414956c2eca0d807f07ee2c91e`.
- Tested current-main release server executable SHA-256:
  `F646C5AC27355180E1C9A690BBD16B091797925BE3285AE278CC7F97C61311EE`.
- Local verification passed on Windows: the locked release build completed,
  then the process regression passed 3/3 fixtures (12 acknowledged writes),
  with property ownership and vector hits verified after both restart paths.

## Root cause

In the v0.2.9 `try_load_state` implementation, `max_u32` started at `0`, then
the loader always set `next_u32` to `max_u32 + 1`. An empty `nodes.bin` thus
started journal replay at internal key `1`, although replaying those same
nodes from a fresh database assigned keys from `0`. The client IDs and vector
metadata could still be recovered, but the shifted ID-to-key assignments no
longer matched the projection's persisted key ownership. SQLite's last-write-
wins property projection then hydrated a recovered node from a different
node's row.

Current source corrects the empty-snapshot case and derives the next key from
the maximum key actually present in a populated snapshot. The required
invariant is that node records, vector metadata, and projected properties use
the same public-ID-to-`u32` mapping on both snapshot load and journal replay.

## Why detection failed

The earlier direct-`Storage` harness passed, but it did not exercise the exact
standalone REST process lifecycle with a fresh empty snapshot, acknowledged
writes, forced process termination, and recovery through the release binary.
That left the release-level path unverified. The new test runs the built
standalone artifact, not an in-process router or storage fixture.

## Prevention

- Preserve the node identity mapping across snapshot loading, full/tail journal
  replay, vector metadata, and projection rebuild.
- Keep the REST process regression in the server-distribution matrix for each
  supported host OS; use distinct property tags and orthogonal vectors so
  identity and property drift are independently visible.
- Keep historical release artifacts unchanged and record exact source and
  binary hashes when making recovery claims.

## Verification scope

The local Windows release test covers three fresh default-retention databases,
hard process termination after acknowledged writes, and graceful checkpoint
plus reopen. CI runs the same scenario on Linux, Windows, and macOS targets.
This verifies the process recovery path and public REST responses; it does not
change or retroactively repair the historical v0.2.9 release artifact.
