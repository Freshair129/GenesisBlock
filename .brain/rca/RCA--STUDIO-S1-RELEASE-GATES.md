---
version: "0.1.0b"
created_at: "2026-10-15T09:28:05+07:00,Codex,working-tree"
last_update: "2026-10-15T10:34:35+07:00,Codex"
status: open
superseded_by: null
attributes:
  doc_type: "rca"
  domain: "genesis-studio-desktop"
  scope: "S1 packaged parity and release evidence"
  issue: 231
---

# RCA - Genesis Studio S1 release gates

Issue [#231](https://github.com/Freshair129/GenesisBlock/issues/231) tracks a
release-readiness evidence gap. It does not assert a production outage.

## Symptom

Studio S1 is beta implemented, but the repository does not yet prove that a
packaged local Tauri session and remote REST session return equivalent results
from one persisted fixture. Packaged native-picker handling, cross-platform
package/startup, and independent architecture/security review evidence are also
absent.

## Evidence

- `docs/SPEC--GENESIS-STUDIO-DESKTOP.md` §25 marks the packaged
  local/remote parity drill and independent-review/cross-platform evidence as
  incomplete and keeps release exit open.
- `studio/src/transports/contract.test.ts` applies its common contract only to
  `createMockTransport`.
- `studio/e2e/studio.spec.ts` exercises the deterministic presentation fixture,
  not a packaged local database and remote REST server.
- `studio/src-tauri/src/lib.rs` validates that the selected root exists and is a
  directory before `Storage::open`, but had no direct path-validation tests.
- `studio/src-tauri/tauri.conf.json` had bundling disabled, and there was no
  Studio-specific Windows/macOS/Linux installer matrix.
- Existing engine and REST tests cover their respective boundaries separately;
  they did not compare both against the same reopened persisted fixture.
- The first package matrix run ([run 38675972613](https://github.com/Freshair129/GenesisBlock/actions/runs/38675972613)) built and uploaded macOS DMG and Linux DEB packages. The Windows MSI job compiled and passed tests but failed in bundling with `Couldn't find a .ico icon`: `icons/icon.ico` existed, but `bundle.icon` did not reference it. The config now lists the existing PNG, ICNS, and ICO assets; the follow-up run is pending.

## Root cause

The beta shell, mock transport, local Tauri commands, and REST routes were
verified in separate test layers. No shared persisted-fixture drill connected
the local and remote boundaries, and the release evidence checklist had no
packaged picker or cross-platform package/startup result to reference.

## Why detection failed

The common frontend contract was registered for the mock transport only, while
the Playwright fixture intentionally remained presentation-only. The spec
preserved the release gates as open, but those separate checks could pass
without assembling evidence for the packaged S1 acceptance criteria.

## Prevention and correction

- Compare the local read-only boundary and actual Studio REST routes against
  the same persisted fixture, including a registered relational schema and
  named query, sequentially to honor exclusive data-root ownership.
- Test blank, missing, file, and valid directory paths through the helper that
  runs before local storage opens.
- Build Windows MSI, macOS DMG, and Linux DEB artifacts in the dedicated
  Studio packaging workflow; record each platform's result before changing the
  release gate.
- Explicitly list the existing platform icon files in `bundle.icon` when
  enabling Tauri packaging; the first Windows CI build caught the omitted
  reference.
- Keep packaged Tauri parity, native-picker interaction, cross-platform
  package/startup, and independent review gates open until their evidence is
  produced and linked in §25.
- Record review finding disposition; do not close the review gate with an
  unresolved P0/P1 finding unless the risk is explicitly accepted.
- Record platform, artifact, test result, and independent finding disposition;
  do not infer package or review evidence from source builds or mock E2E.

## Verification scope and limitations

The regression test exercises a read-only `Storage` open and the Axum router in
process against one temporary persisted database. It does not launch the
packaged desktop app, send requests over TCP, interact with the native folder
dialog, prove startup from an installer, or replace the independent
architecture/security review. The first platform packaging run built macOS and
Linux artifacts but failed to build the Windows MSI because the Tauri bundle
configuration omitted the existing icons. The config correction is awaiting a
rerun. These unsigned development artifacts do not establish the S4
signing/update policy. Those remain release gates.

The current Windows checkout has no WiX `candle.exe` or `light.exe`, so it
cannot locally produce the MSI installer. This does not block the Windows,
macOS, and Linux package matrix from running in GitHub Actions. The Windows MSI
is pending the corrected run; package startup evidence remains uncollected.
