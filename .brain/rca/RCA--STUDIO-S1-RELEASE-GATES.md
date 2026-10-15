---
version: "0.2.0b"
created_at: "2026-10-15T09:28:05+07:00,Codex,working-tree"
last_update: "2026-10-15T12:13:35+07:00,Codex"
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
from one persisted fixture. Packaged native-picker handling, installed-app
startup on the declared platforms, and independent architecture/security
review evidence are absent. Cross-platform package build evidence is now
available from the Studio CI matrix.

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
- The first package matrix run ([run 38675972613](https://github.com/Freshair129/GenesisBlock/actions/runs/38675972613)) built and uploaded macOS DMG and Linux DEB packages. The Windows MSI job compiled and passed tests but failed in bundling with `Couldn't find a .ico icon`: `icons/icon.ico` existed, but `bundle.icon` did not reference it. The config now lists the existing PNG, ICNS, and ICO assets. The corrected matrix ([run 38677091691](https://github.com/Freshair129/GenesisBlock/actions/runs/38677091691)) passed all three platform bundle jobs and uploaded the MSI, DMG, and DEB.

- `studio/src-tauri/tauri.conf.json` production `connect-src` permits the local app and Tauri IPC but no remote server origin, while `createRemoteTransport` used WebView `fetch` for the configured remote base URL. Browser Vitest does not enforce the packaged Tauri CSP.
- The packaged transport change routes remote operations through Tauri IPC to a Rust client with fixed read-only operation/method/path mappings, URL and query validation, redirects disabled, and same-origin token forwarding. Frontend tests cover native bridge selection, token/configuration handoff, disconnect cleanup, and cleanup after capability-handshake failure. The Tauri Rust library suite passed 9/9 after a regression exposed and fixed bracketed IPv6 loopback validation; the strengthened persisted-fixture parity test passed 1/1, and the read-only REST server binary built successfully.
- The packaging workflow now includes install-and-launch smoke steps and uploaded logs for all three package types. Its Windows job also seeds a persisted graph/vector/relational fixture, checks missing/non-directory paths, automates both cancel and successful native folder selection, and reads the same fixture through the installed app in local mode and through an authenticated loopback REST server in remote mode. After UI read flows complete, the probe uses the packaged Tauri invoke bridge to collect explicit local and remote native-operation snapshots, then compares declared read capabilities/limits and S1 results, including status/frontier, graph scene and expansion, vector collections, schemas, entity inspection, read-only HQL, and a registered named query. UI-visible results are compared separately. The workflow code is new and has not yet run on this revision; these are pending probes, not evidence.
- The first Studio Rust test compile reached the new Tauri test target but failed because its CSP test lacked a `Value` import. After fixing that test import, the suite exposed that `Url::host_str()` returns a bracketed IPv6 host, causing loopback HTTP to be rejected; validation now strips exactly one bracket pair before parsing the address. The full Tauri library suite passed 9/9. The strengthened root parity test passed 1/1 and the `genesis-db-server` build passed. These are source/test/build checks, not installed-package evidence.

## Root cause

The beta shell, mock transport, local Tauri commands, and REST routes were
verified in separate test layers. No shared persisted-fixture drill connected
the local and remote boundaries, and the release evidence checklist had no
packaged picker or cross-platform package/startup result to reference. The
packaged production CSP also denied remote origins while the renderer's remote
adapter used `fetch`, so successful browser tests did not prove the remote mode
could run in the packaged WebView.

## Why detection failed

The common frontend contract was registered for the mock transport only, while
the Playwright fixture intentionally remained presentation-only. The spec
preserved the release gates as open, but those separate checks could pass
without assembling evidence for the packaged S1 acceptance criteria. Package
CI built installers but did not launch them or exercise remote networking under
the production CSP; browser test environments do not apply Tauri's configured
CSP.

## Prevention and correction

- Compare the local read-only boundary and actual Studio REST routes against
  the same persisted fixture, including a registered relational schema and
  named query, sequentially to honor exclusive data-root ownership.
- Test blank, missing, file, and valid directory paths through the helper that
  runs before local storage opens.
- Build Windows MSI, macOS DMG, and Linux DEB artifacts in the dedicated
  Studio packaging workflow; record each platform's result before changing the
  release gate.
- Install and launch the built MSI, DMG, and DEB on their matching CI runners
  and keep the smoke bounded with timeouts. On Windows, use UI Automation to
  assert missing/non-directory errors, cancel the actual folder dialog, then
  select the persisted parity fixture. Read it through packaged local mode,
  close the local handle, start the authenticated REST server over the same
  root, and compare supported S1 results from packaged remote mode. Record the
  successful run artifacts before changing any release gate.
- Explicitly list the existing platform icon files in `bundle.icon` when
  enabling Tauri packaging; the first Windows CI build caught the omitted
  reference.
- Route packaged remote REST calls through a Rust native client over Tauri IPC;
  keep the production WebView `connect-src` restricted to app and IPC origins.
- Expose only fixed read-only route/method variants, validate URL scheme/host/
  port/path and graph query values, compare every target with the configured
  origin, disable redirects, and clear the in-memory token on disconnect or a
  failed initial capabilities handshake.
- Repeatedly decode path segments during validation to reject nested encoded
  traversal, separators, query/fragment delimiters, and invalid encoded bytes;
  reject unsafe entity-ID segments; and bound native JSON and error response
  buffering (32 MiB and 64 KiB respectively).
- Test URL rejection, route/method/query mapping, bearer forwarding, redirect
  refusal, bounded chunked responses, unchanged CSP, native bridge selection,
  and handshake cleanup. Independent review must include those path/ID/body
  boundaries and token lifecycle before the release gate can close.
- Keep packaged Tauri parity, native folder-picker interaction, startup from
  installed packages on each declared platform, and independent review gates
  open until their evidence is produced and linked in §25.
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
configuration omitted the existing icons. After explicitly configuring the
existing assets, the corrected matrix built all three packages and uploaded
them. These unsigned development artifacts do not establish the S4
signing/update policy or prove startup from installed packages. Those remain
release gates.

The native remote bridge code has frontend Vitest/build coverage. Its Rust URL,
allowlist, token-forwarding, redirect, response-size, and CSP tests passed 9/9
in the Tauri library target. The strengthened persisted-fixture contract test
passed 1/1 and the read-only REST server binary build passed. The packaged
fixture seeder, installed startup probes, and Windows UIA selection/parity
script pass local syntax checks only; the new workflow has not run. No
packaged picker-selection, parity, or new startup result is claimed.
Independent architecture/security review also remains open pending the
committed diff review and current-head CI.

The current Windows checkout has no WiX `candle.exe` or `light.exe`, so it
cannot locally produce the MSI installer. GitHub Actions produced Windows,
macOS, and Linux package artifacts in run 38677091691. Startup evidence for
installed packages remains uncollected.
