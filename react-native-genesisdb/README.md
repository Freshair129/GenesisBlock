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
Android bridge method-for-method. It is **not yet a drop-in `pod install`**,
for the same reason `android/build.gradle` currently references a
`genesisdb-android` Maven coordinate that isn't published either — two
pieces this package doesn't (and structurally can't) solve on its own:

1. **`GenesisBlockDB.xcframework` isn't published as a release asset yet.**
   The podspec has nothing to `s.vendored_frameworks` against until it is —
   see `docs/SPEC--MOBILE-SDK.md` §B-1's "Not yet done". A local monorepo
   build can assemble one via the `ios-xcframework` CI job's
   `xcodebuild -create-xcframework` command.
2. **CocoaPods cannot express a dependency on a Swift Package.**
   `GenesisDbModule.swift` does `import GenesisDB` / `import GenesisDBTypes`
   (the `ios/genesisdb` package's products) — a podspec has no mechanism to
   pull those in itself. A consuming app must add `ios/genesisdb` as a Swift
   Package dependency directly in its own Xcode project (Xcode's "Add
   Package Dependency", pointing at `../ios/genesisdb` for now, or the
   package's eventual published Git URL), **in addition to** running
   `pod install` for `react-native-genesisdb`. This is a standard, documented
   CocoaPods+SPM coexistence pattern — not a workaround — but it is a real
   extra manual step any integrator needs to know about today.

Neither of these is verified by this monorepo's CI: there is no real RN host
app here to resolve either dependency against, the same host-only carve-out
already documented for the Android side and for `ios/genesisdb` itself.

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
