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
| iOS | Stub only. Every method rejects with a `GENESISDB_IOS_NOT_IMPLEMENTED` error until Phase B-1 (the `GenesisBlockDB.xcframework` + Swift wrapper) ships. `pod install` and autolinking still succeed — the package just isn't functional on iOS yet. |

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
