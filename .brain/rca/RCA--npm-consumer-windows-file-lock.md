# RCA: Windows Registry Consumer Smoke Retains the Native Database Handle

## Status / Date

Root cause confirmed; subprocess isolation passed locally, hosted matrix pending / 2026-09-27.

## Symptom

The new clean npm consumer smoke installed the public
`@freshair129/gks-genesis-block-native@0.2.7` package on Windows, completed an
MCP handshake with five tools, and wrote a node through N-API. It then exited
with code 1 while deleting its temporary consumer directory:

```text
EBUSY: resource busy or locked, unlink '...\native-consumer-db\projection.sqlite'
```

In PowerShell, set PACKAGE_VERSION to 0.2.7 and run
`node scripts/npm-registry-consumer-smoke.mjs`.

## Evidence

- npm installed the published package successfully from the public registry.
- The smoke printed `registry MCP handshake (0.2.7): 5 tools`.
- The native database write and state save completed and printed
  `registry N-API round trip: registry-npm-smoke`.
- Cleanup then failed while removing the SQLite projection file on Windows.
- `index.d.ts` declares `GenesisDatabase.open()` but exposes no close method;
  the N-API instance can remain alive until its Node process exits.
- The same file-lock error is specific to cleanup after the native write; the
  MCP child had already exited cleanly.

## Root Cause

The smoke kept the native `GenesisDatabase` object alive in the parent Node
process and attempted to remove its SQLite-backed directory before that process
exited. Windows prevents unlinking `projection.sqlite` while the native
database handle still owns it. The published API does not expose a deterministic
close operation for this test to call.

## Why the Issue Escaped Detection

Earlier public-package verification exercised the handshake and native write
but did not remove the database directory in the same process. This new
cross-platform harness added eager temporary-directory cleanup, which exposed
the Windows file-handle lifetime.

## Fix (Decided)

Run only the N-API write smoke in a short-lived child Node process. When that
process exits, its native handles are released; the parent can then remove the
temporary consumer directory on every platform. Keep the public npm install,
MCP handshake, and native write assertions unchanged.

## Outcome (Measured)

Local Windows rerun passed: public v0.2.7 install, five-tool MCP handshake,
N-API write, and temporary-directory cleanup all exited successfully. Hosted
Linux/Windows/macOS matrix validation is pending.

## Proposed Prevention

For Windows consumer fixtures that use a native object without an explicit
close API, end the process owning that object before deleting its data directory.
