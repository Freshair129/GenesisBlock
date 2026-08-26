# react-native-genesisdb

Embedded GenesisBlockDB for React Native (MARK XVI Phase B-3) — local-first
hybrid semantic-graph engine, no server required. See
[docs/SPEC--MOBILE-SDK.md](../docs/SPEC--MOBILE-SDK.md) §B-3.

```ts
import { GenesisDB } from 'react-native-genesisdb';

const db = await GenesisDB.open(`${DocumentDirectoryPath}/genesisdb`);
const node = await db.addNode({ labels: ['Person'], props: { name: 'Ada' } });
const ctx = await db.retrieveContext(node.id, 'H1');
await db.close();
```

## Platform status

| Platform | Status |
|---|---|
| Android | **Working, but needs a GitHub token.** Bridges to `dev.genesisblock:genesisdb-android` (Phase B-2), which is published to GitHub Packages — authenticated even for public repos. See [Installation — Android](#installation--android). |
| iOS | **Not usable from npm today.** `GenesisDbModule.swift` is real code calling the `ios/genesisdb` Swift package (Phase B-1), but that package is not shipped in this npm tarball, so the module cannot compile for a consumer who installed from npm. Works only from a monorepo checkout. See [iOS integration status](#ios-integration-status). |

## Installation — Android

`npm install react-native-genesisdb` is not sufficient on its own: the native
`.aar` this package bridges to lives in **GitHub Packages**, which requires
authentication even though the repository is public. Without a token, the
Android build fails to resolve `dev.genesisblock:genesisdb-android:0.1.0`.

Create a GitHub personal access token with **only the `read:packages` scope**,
then add it to `~/.gradle/gradle.properties` (user-level — do not commit it to
your app's repo):

```properties
gpr.user=YOUR_GITHUB_USERNAME
gpr.key=ghp_yourTokenHere
```

`android/build.gradle` reads those two properties (falling back to the
`GITHUB_ACTOR` / `GITHUB_TOKEN` environment variables, which CI already sets).

> **Why a token at all?** GitHub Packages was chosen over Maven Central
> because it needed no new account or GPG signing setup — the tradeoff, noted
> in [`android/README.md`](../android/README.md), is exactly this consumer-side
> token requirement. Removing it means either publishing to Maven Central or
> bundling the `.so` slices into this package directly; both are open items.

## iOS integration status

`GenesisDbModule.swift` is real code, not a stub, and its logic mirrors the
Android bridge method-for-method. It is **not yet a drop-in `pod install`**
— one piece this package doesn't (and structurally can't) solve on its own:

1. ~~`GenesisBlockDB.xcframework` needs a `prepare_command` to fetch it during
   `pod install`.~~ **Done (issue #125)**: `react-native-genesisdb.podspec`'s
   `prepare_command` now downloads, SHA256-verifies, and unzips the
   [v0.2.0 release](https://github.com/Freshair129/GenesisBlock/releases/tag/v0.2.0)'s
   `GenesisBlockDB.xcframework.zip` automatically — no manual step. (An
   earlier version of this doc also said the podspec would need a CocoaPods
   Trunk publish first; that was wrong — RN autolinking's
   `use_native_modules!` picks up this podspec directly from `node_modules`,
   the same way virtually every third-party RN native module ships its iOS
   half, with zero Trunk publish involved.)
2. **The Swift package this module imports is not in the npm tarball.**
   `GenesisDbModule.swift` does `import GenesisDB` / `import GenesisDBTypes`
   — products of the monorepo's `ios/genesisdb` package. This npm package
   ships `src`, `lib`, `android`, `ios` (its *own* RN module directory) and
   the podspec; `ios/genesisdb` lives one level above the package root, so
   npm cannot include it. The podspec's `prepare_command` fetches the
   `GenesisBlockDB.xcframework` (module name `GenesisBlockDB`, the raw C
   ABI) — which is **not** the same module the Swift bridge imports.

   The practical consequence, stated plainly: **`pod install` succeeds and
   then the build fails with "no such module 'GenesisDB'"** for anyone who
   installed this package from npm. CocoaPods also cannot express a
   dependency on a Swift Package, so there is no podspec-level fix.

   Working today only from a **monorepo checkout**, where a consuming app can
   add `../ios/genesisdb` via Xcode's "Add Package Dependency". Issue #125
   deliberately deferred giving `ios/genesisdb` its own root-level repo,
   which is what would make a real published SPM URL — and therefore a
   working npm-installed iOS path — possible.

   Two candidate fixes, neither done yet: give `ios/genesisdb` its own repo
   and depend on it by URL, or vendor its Swift sources into this package's
   `ios/` directory at publish time so `s.source_files` compiles them
   directly against the xcframework.

Neither this package's own `pod install`+SPM combination nor `android/build.gradle`'s
Maven resolution is verified by this monorepo's CI: there is no real RN host
app here to resolve any of them against, the same host-only carve-out already
documented for the Android side and for `ios/genesisdb` itself. That gap is
why both breakages above shipped unnoticed in `0.1.0`.

## Publishing

`.github/workflows/release.yml`'s `rn-npm-publish` job publishes this package
to npm (unscoped `react-native-genesisdb`, `--tag beta`) on every `v*` tag
push, reusing the same `NPM_TOKEN` secret the main native-addon package
publishes with (issue #125).

That job has run: **`react-native-genesisdb@0.1.0` is live on npm** (first
published from the `v0.2.2` tag). Re-publishing an unchanged version is
correctly rejected by the registry with `403 You cannot publish over the
previously published versions`, so later tags no-op unless the version in
`package.json` is bumped.

## Wire format

All types in `src/types.ts` are **snake_case**, matching the engine's raw
`serde_json` output (`valid_from`, `query_vector`, ...) — not the camelCase
used by the Node addon's `index.d.ts` (a napi-rs-only convention). See the
doc comment at the top of `src/types.ts` for why a generic camelCase
conversion layer was deliberately not built (it would corrupt keys inside the
opaque `props` field).

## Testing

`src/__tests__/index.test.ts` runs under plain Jest against a stubbed
`react-native` module (`src/__mocks__/react-native.ts`) — no native build,
no simulator/emulator, no RN runtime. Run with `npm test`.

The `android/` and `ios/` native module sources are validated by building
them inside a real RN host app; that is out of scope for this monorepo's CI
(same host-only carve-out as the rest of the mobile SDK — see
`.github/workflows/mobile-build.yml`).
