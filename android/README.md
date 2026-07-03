# genesisdb-android

Embedded GenesisBlockDB for Android (MARK XVI Phase B-2). Wraps the JNI bridge
in `src/jni.rs` — see [docs/SPEC--MOBILE-SDK.md](../docs/SPEC--MOBILE-SDK.md) §B-2.

```kotlin
val db = GenesisDB.open(context.filesDir.resolve("genesisdb").absolutePath)
val node = db.addNode(NodeInput(labels = listOf("Person"), props = null))
val ctx = db.retrieveContext(node.id, tier = "H1")
db.close()
```

## Building

This module never invokes `cargo` itself — it links a prebuilt
`libgenesis_block_native.so` per ABI dropped into
`genesisdb/src/main/jniLibs/{arm64-v8a,armeabi-v7a}/`.

- **CI** (`.github/workflows/mobile-build.yml`, job `android-aar`): builds the
  `.so` slices via `cargo ndk` with `--features "mobile ffi android-jni"`,
  stages them into `jniLibs/`, then runs `gradle :genesisdb:assembleRelease`
  and uploads the `.aar`.
- **Local dev** (requires a Mac/Linux box with the Android NDK — this cannot
  be exercised on the Windows dev host): run
  `ANDROID_NDK_HOME=<path> ./scripts/gen-android-jnilibs.sh [debug|release]`
  from the repo root, then `gradle :genesisdb:assembleRelease` from `android/`.

## Wire format gotcha

The JNI bridge serializes the *same* `serde`-derived Rust structs the REST
server uses (`NodeInput`, `NodeOutput`, ...), with **no** `rename_all`
attribute — so the JSON crossing the JNI boundary is **snake_case**
(`valid_from`, `query_vector`, ...), not the camelCase seen in the Node addon's
`index.d.ts` (that's a napi-rs-specific binding convention, not the wire
format). `Types.kt` carries an explicit `@SerialName` per field for this
reason — if you add a field, mirror the exact Rust field name in the
`@SerialName`, not the napi/TS name.
