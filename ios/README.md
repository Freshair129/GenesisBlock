# GenesisBlockDB — iOS SDK

Embedded GenesisBlockDB for iOS/macOS (MARK XVI Phase B-1). Wraps the C ABI in
`src/ffi.rs` — see [docs/SPEC--MOBILE-SDK.md](../docs/SPEC--MOBILE-SDK.md) §B-1.
The Android SDK (`../android/`) is B-2's equivalent; this package deliberately
mirrors its structure and conventions (wire types, wire-format tests, README
shape) — most of the design notes below have a one-to-one Kotlin counterpart.

```swift
let db = try GenesisDB.open(path: url)
let node = try await db.addNode(NodeInput(labels: ["Person"]))
let ctx = try await db.retrieveContext(targetId: node.id, tier: "H1")
await db.close()
```

## Package layout

```
genesisdb/
├── Package.swift
├── Sources/
│   ├── GenesisDBTypes/    Codable wire types — zero C dependency
│   ├── CGenesisDBFFI/     system-library module wrapping include/genesisdb.h
│   └── GenesisDB/         the actor wrapper (GenesisDB.swift)
└── Tests/
    ├── GenesisDBTypesTests/  wire-format tests (no native lib needed)
    └── GenesisDBTests/       real round trip against the compiled engine
```

## Building

Unlike the Android module (which only ever links a prebuilt `.so`), this
package's `GenesisDB` target links `libgenesis_block_native.a` directly via
`Package.swift`'s `unsafeFlags` — deliberately NOT the shape of the eventual
published SPM package (see the warning at the top of `Package.swift`). The
Rust lib must already be built before running `swift build`/`swift test`:

```bash
# From the repo root — deliberately NO --target, so cargo builds for the
# HOST's own default triple (whatever that is) rather than a cross-compiled
# iOS slice, which couldn't execute on the build machine.
cargo build --no-default-features --features "mobile ffi"

# The C header is committed nowhere under ios/ (see
# Sources/CGenesisDBFFI/module.modulemap) — copy the freshly generated one in
# before every build/test:
mkdir -p ios/genesisdb/Sources/CGenesisDBFFI/include
cp include/genesisdb.h ios/genesisdb/Sources/CGenesisDBFFI/include/genesisdb.h

cd ios/genesisdb
swift test
```

`GENESISDB_RUST_LIB_DIR` overrides the lib search path if you built a
different profile than the default (`../../target/debug`).

This all requires a macOS/Xcode toolchain and **cannot be exercised on the
Windows dev host** — the same carve-out `android/README.md` documents for the
NDK. CI (`.github/workflows/mobile-build.yml`, `macos-latest` runners) is
where every command above actually runs:

- **`ios-swift-tests`** — runs both test targets. `GenesisDBTypesTests` needs
  no Rust lib at all (mirrors `android-jvm-tests`'s no-native-lib property
  exactly); `GenesisDBTests` builds the host-arch static lib first and runs a
  *real* `addNode`/`retrieveContext`/`search` round trip against the compiled
  engine — the Phase B DoD item "addNode + retrieveContext round-trip in a
  Swift test target".
- **`ios-xcframework`** — builds the `aarch64-apple-ios` + `aarch64-apple-ios-sim`
  static libs (same command `ios-build` already ran) and assembles
  `GenesisBlockDB.xcframework` via `xcodebuild -create-xcframework`, uploaded
  as a build artifact — DoD item "`GenesisBlockDB.xcframework` builds for
  `aarch64-apple-ios` + `aarch64-apple-ios-sim`".

## Wire format gotcha

The C ABI serializes the *same* `serde`-derived Rust structs the REST server
and the Android JNI bridge use (`NodeInput`, `NodeOutput`, ...), with **no**
`rename_all` attribute — so the JSON crossing the C boundary is **snake_case**
(`valid_from`, `query_vector`, ...), not the camelCase seen in the Node
addon's `index.d.ts` (a napi-rs-specific binding convention that does not
apply here). `Types.swift` carries an explicit `CodingKeys` per struct for
this reason — if you add a field, mirror the exact Rust field name, not the
napi/TS name. (`Types.swift`'s `ContextPackage` includes `coverage:
CoverageReport` — that field was missing from the Android SDK's `Types.kt`
until this same change added it there too, since `ignoreUnknownKeys`-style
leniency was silently dropping it for every Android caller.)

## Not yet done

- Publishing `GenesisBlockDB.xcframework` as a versioned GitHub release asset
  and swapping `Package.swift` to the `.binaryTarget(url:, checksum:)` form a
  real external consumer would use.
- Literal on-device/Xcode-project acceptance (`import GenesisBlockDB` in a
  blank Xcode project, running on a physical device) — out of scope for this
  monorepo's CI, the same host-only carve-out `android/README.md` and
  `react-native-genesisdb`'s iOS stub already document for their own
  device-only checks.
- `react-native-genesisdb`'s iOS module (`../react-native-genesisdb/ios/`) is
  still the B-1-pending stub — wiring it to this package is the natural next
  step once the above lands.
