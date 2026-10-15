---
version: "0.3.0b"
created_at: "2026-10-15T09:28:05+07:00,Codex,working-tree"
last_update: "2026-10-15T12:51:10+07:00,Codex"
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
from one persisted fixture. Packaged Windows picker/path handling and local UI
reads now have evidence, but no installed UI reached remote mode, so
remote-origin behavior under the production CSP remains unverified.
Independent architecture/security review acceptance remains open. In package
run [38698802743](https://github.com/Freshair129/GenesisBlock/actions/runs/38698802743)
at head `aaf71c63cb7ed538e3e2db7da1ee2472a5dbd5ad`, Linux package/startup passed
and the ordinary Windows MSI installed, launched for ten seconds, and
uninstalled. The Windows UI smoke passed missing/non-directory path handling,
picker cancellation and selection, local connection, and local reads, then
failed at the return-to-Connections button: the runner is 1024px wide while the
responsive CSS hid that button below 1050px. It did not start remote mode, so
packaged remote-origin behavior and same-fixture parity remain unproven.
The macOS package job was still pending when this evidence was recorded.

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
- Studio packaging run [38686388595](https://github.com/Freshair129/GenesisBlock/actions/runs/38686388595), at head `5419a91cca67cd5aca67deea130c031d2eaa84f4`, completed all three package jobs. The Linux DEB package installed and launched successfully; the macOS DMG package installed and launched successfully; the Windows MSI installer completed successfully but the smoke failed before UI interaction because Playwright `connectOverCDP` received `ECONNREFUSED 127.0.0.1:50705`. The Windows fixture seed and REST server build steps passed, but no packaged picker, local/remote UI parity, remote-mode CSP result, or startup-screen result is claimed. The [MSI](https://github.com/Freshair129/GenesisBlock/actions/runs/38686388595/artifacts/11950739509), [DMG](https://github.com/Freshair129/GenesisBlock/actions/runs/38686388595/artifacts/11950784409), [DEB](https://github.com/Freshair129/GenesisBlock/actions/runs/38686388595/artifacts/11950242206), and [smoke evidence](https://github.com/Freshair129/GenesisBlock/actions/runs/38686388595/artifacts/11950994174) are retained as workflow artifacts.
- Follow-up package run [38698802743](https://github.com/Freshair129/GenesisBlock/actions/runs/38698802743), head `aaf71c63cb7ed538e3e2db7da1ee2472a5dbd5ad`: Linux DEB install/startup passed; Windows persisted-fixture parity test, REST server build, Tauri shell tests, MSI build, ordinary MSI install/start/uninstall, missing/non-directory validation, picker cancel/select, local UI reads, and native local reads passed. The UI smoke then timed out finding the `Connections` button. The Windows Installer evidence reports 1024x768; `styles.css` hid `.disconnect` at max-width 1050px. Thus local interaction was reached, but the smoke never signaled local disconnect or started the remote server. The UI now retains an accessible icon-only connection button at this narrow desktop width. Windows smoke evidence is [artifact 11956860311](https://github.com/Freshair129/GenesisBlock/actions/runs/38698802743/artifacts/11956860311), the ordinary MSI evidence is included there, and the Linux DEB is [artifact 11955897591](https://github.com/Freshair129/GenesisBlock/actions/runs/38698802743/artifacts/11955897591). The macOS job and rerun of this UI fix remain pending.
- The first Studio Rust test compile reached the new Tauri test target but failed because its CSP test lacked a `Value` import. After fixing that test import, the suite exposed that `Url::host_str()` returns a bracketed IPv6 host, causing loopback HTTP to be rejected; validation now strips exactly one bracket pair before parsing the address. The full Tauri library suite passed 9/9. The strengthened root parity test passed 1/1 and the `genesis-db-server` build passed. These are source/test/build checks, not installed-package evidence.
- A read-only review of head `8463222b4c853050437f96f1dbb6199a710a7e9d` found a credential-routing gap: `RemoteConnection::new` used reqwest's default client, which enables system proxies, while the bridge permits loopback HTTP and attaches the bearer authorization header. If `HTTP_PROXY`/`ALL_PROXY` is set without a matching `NO_PROXY` exclusion, a loopback bearer request could be routed in cleartext through the proxy. The bridge now applies `no_proxy()` only to validated HTTP URLs (which the validator restricts to loopback), while retaining the default system-proxy behavior for HTTPS. The focused regression `cargo test --manifest-path studio/src-tauri/Cargo.toml --locked --lib --jobs 1 loopback_http_bypasses_proxies_while_https_keeps_proxy_support` passed: 1 passed, 0 failed, 9 filtered. It observed loopback HTTP bypassing an explicit local proxy and HTTPS using `CONNECT example.com:443`.
- The Windows packaged smoke failure in run 38698802743 is a responsive navigation defect, not an MSI install or WebView2 startup failure. At the runner's 1024px screen width, the max-width:1050px rule hid the only Connections control. The UI now exposes that control as an accessible icon button in the narrow desktop sidebar; the next package run must verify the strict picker/local/remote parity flow.

## Root cause

The beta shell, mock transport, local Tauri commands, and REST routes were
verified in separate test layers. No shared persisted-fixture drill connected
the local and remote boundaries, and the release evidence checklist had no
packaged picker or cross-platform package/startup result to reference. The
packaged production CSP also denied remote origins while the renderer's remote
adapter used `fetch`, so successful browser tests did not prove the remote mode
could run in the packaged WebView.

The first packaged Windows UI-smoke attempt also had a harness configuration
defect: it set `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` after launch, but Wry
0.55.1 passes browser arguments through WebView2 environment options and does
not consume that variable for this window. Consequently the configured CDP
endpoint never opened and Playwright failed before the smoke could exercise the
picker or packaged data paths. This is a test setup failure; it neither passes
nor demonstrates a product startup failure.

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
  and keep the smoke bounded with timeouts. The latest Linux and macOS probes
  passed; Windows MSI installation passed but its UI/CDP smoke did not reach
  the app UI. The follow-up builds a separate smoke-only Windows MSI with
  explicit `app.windows[0].additionalBrowserArgs` and a per-run loopback CDP
  port. Stage the ordinary MSI artifact before creating that variant and upload
  only the staged ordinary MSI. On Windows, use UI Automation to
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
- For validated loopback HTTP, disable system and configured proxies so the
  bearer header stays on the loopback connection; retain default system-proxy
  routing for HTTPS remote servers. Regress both branches with an explicit local
  proxy probe before accepting the fix.
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
- Keep the Connections action available and accessibly named at supported
  desktop widths that collapse the sidebar; package smoke exercises it after
  local reads before starting the remote REST session.

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
passed 1/1 and the read-only REST server binary build passed. Packaging run
[38690064356](https://github.com/Freshair129/GenesisBlock/actions/runs/38690064356)
passed Linux DEB and macOS DMG installation/startup probes and uploaded all
three ordinary package artifacts. Its Windows MSI installed, stayed alive for
ten seconds, and uninstalled successfully, but the instrumented smoke timed out
waiting for the local connection chip. At that point the artifact did not retain
DOM/alert evidence, so the failure could not be attributed to transport or UI
state.

The follow-up at head `afced2e2bd3315cb87597a8b5903bcde931caf13` separated
capability state from the unrelated initial data reads and retained a strict
`local / local` assertion. Fresh packaging run
[38694137032](https://github.com/Freshair129/GenesisBlock/actions/runs/38694137032)
passed Linux DEB and macOS DMG installation/startup, Windows parity fixture
seeding, REST server build, Tauri shell tests, MSI build, and ordinary MSI
install/start/uninstall. The staged Windows MSI SHA-256 was
`e0ff990ee489e866f7e4557a3efb2daf6f1e63c87e9592448f0cb28b04716061`; the
installed `C:\Program Files\Genesis Studio\genesis-studio.exe` remained alive
for ten seconds, then uninstalled with exit 0. The ordinary package evidence
still sets `visible_ui_verified: false` and `fixture_parity: false`.

The instrumented MSI's CDP endpoint `127.0.0.1:59921` came up. Missing and
non-directory paths, picker cancellation, and selection of the exact persisted
fixture root passed. The strict `local / local` state was reached, and the
visible local read surfaces passed. The failure occurred afterward in the test
probe: `readLocalNativeOperations` called `studio_open_local` again for the
same root after the UI had already opened it. The native command correctly
rejected a second owner with `database is already open by another
GenesisBlockDB process: <fixture-root>`. The probe now calls the existing-session
`studio_local_capabilities` command and no longer reopens the root. The Windows
job failed before remote requests or packaged parity comparison. See the
[Windows evidence artifact
11954183968](https://github.com/Freshair129/GenesisBlock/actions/runs/38694137032/artifacts/11954183968).

The duplicate-open diagnosis explains the latest run's failure; it does not
complete acceptance. The corrected probe needs a fresh Windows package run to
prove native local operations, disconnect, remote REST connection under the
production CSP, and same-fixture parity through the packaged UI. Independent
architecture/security review acceptance also remains open until the exact
revised diff is reviewed and recorded.

The current Windows checkout has no WiX `candle.exe` or `light.exe`, so it
cannot locally produce the MSI installer. GitHub Actions produced MSI, DMG, and
DEB artifacts in run 38694137032. Linux and macOS installed startup and ordinary
Windows MSI process startup/uninstall were observed. Windows picker selection,
validation, local connection, and visible local reads are evidenced on the
instrumented package, while native-operation parity, remote-origin/CSP, and
same-fixture parity remain open.
