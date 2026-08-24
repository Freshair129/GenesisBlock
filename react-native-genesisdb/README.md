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
| Android | Working — bridges to `dev.genesisblock:genesisdb-android` (Phase B-2). |
| iOS | `GenesisDbModule.swift` now calls the real `ios/genesisdb` Swift package (Phase B-1) instead of stub-rejecting every method — but see "iOS integration status" below before assuming `pod install` alone is enough. |

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
2. **CocoaPods cannot express a dependency on a Swift Package.**
   `GenesisDbModule.swift` does `import GenesisDB` / `import GenesisDBTypes`
   (the `ios/genesisdb` package's products) — a podspec has no mechanism to
   pull those in itself. A consuming app must add `ios/genesisdb` as a Swift
   Package dependency directly in its own Xcode project (Xcode's "Add
   Package Dependency", pointing at `../ios/genesisdb` for now — issue #125
   deliberately defers giving `ios/genesisdb` its own root-level repo for a
   "real" published SPM URL), **in addition to** running `pod install` for
   `react-native-genesisdb`. This is a standard, documented CocoaPods+SPM
   coexistence pattern — not a workaround — but it is a real extra manual
   step any integrator needs to know about today.

Neither this package's own `pod install`+SPM combination nor `android/build.gradle`'s
Maven resolution is verified by this monorepo's CI: there is no real RN host
app here to resolve any of them against, the same host-only carve-out already
documented for the Android side and for `ios/genesisdb` itself.

## Publishing

`.github/workflows/release.yml`'s `rn-npm-publish` job publishes this package
to npm (unscoped `react-native-genesisdb`, `--tag beta`) on every `v*` tag
push, reusing the same `NPM_TOKEN` secret the main native-addon package
publishes with (issue #125). As of this writing that job exists but hasn't
run yet — `npm install react-native-genesisdb` doesn't resolve anywhere until
the next tag push triggers it.

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
