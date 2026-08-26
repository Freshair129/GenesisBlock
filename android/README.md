# genesisdb-android

Embedded GenesisBlockDB for Android (MARK XVI Phase B-2). Wraps the JNI bridge
in `src/jni.rs` — see [docs/SPEC--MOBILE-SDK.md](../docs/SPEC--MOBILE-SDK.md) §B-2.

```kotlin
val db = GenesisDB.open(context.filesDir.resolve("genesisdb").absolutePath)
val node = db.addNode(NodeInput(labels = listOf("Person"), props = null))
val ctx = db.retrieveContext(node.id, tier = "H1")
db.close()
```

## ABIs

Three slices ship in the `.aar`, declared by `abiFilters` in
`genesisdb/build.gradle.kts` and built from three Rust targets. The two
spellings are not interchangeable — `cargo ndk` takes the triple, `jniLibs/`
and `abiFilters` take the ABI name:

| Android ABI | Rust target triple | What it is for |
|---|---|---|
| `arm64-v8a` | `aarch64-linux-android` | Every modern physical device |
| `armeabi-v7a` | `armv7-linux-androideabi` | Older 32-bit physical devices |
| `x86_64` | `x86_64-linux-android` | The **emulator** |

`x86_64` was added in **0.1.1**. Before that the `.aar` carried only the two
ARM slices, which meant a consumer on a Windows or Linux dev machine could not
run their app in the standard Android Studio AVD at all — that AVD is x86_64,
and `System.loadLibrary("genesis_block_native")` finds no matching slice, so
the app dies at load time with `UnsatisfiedLinkError`. It is also what any
future emulator-based acceptance job on GitHub's x86_64 runners needs.

The third slice costs roughly +7-10 MiB uncompressed. That is only an
acceptable price because these are stripped `--release` builds: the
`android-publish` job asserts every slice is free of `.debug_*` sections and
under a 40 MiB per-slice ceiling, after a debug `.aar` was once published at
281 MB (see `CHANGELOG.md`).

## Building

This module never invokes `cargo` itself — it links a prebuilt
`libgenesis_block_native.so` per ABI dropped into
`genesisdb/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/`.

- **CI** (`.github/workflows/mobile-build.yml`, job `android-aar`): builds the
  `.so` slices via `cargo ndk` with `--features "mobile ffi android-jni"`,
  stages them into `jniLibs/`, then runs `gradle :genesisdb:assembleRelease`
  and uploads the `.aar`.
- **Local dev** (requires a Mac/Linux box with the Android NDK — this cannot
  be exercised on the Windows dev host): run
  `ANDROID_NDK_HOME=<path> ./scripts/gen-android-jnilibs.sh [debug|release]`
  from the repo root, then `gradle :genesisdb:assembleRelease` from `android/`.
- **Prebuilt `.aar`**: a real, CI-built `.aar` (version 0.1.0, engine 0.2.0) is
  attached to the [v0.2.0 GitHub Release](https://github.com/Freshair129/GenesisBlock/releases/tag/v0.2.0)
  as `genesisdb-android-0.1.0.aar`. This is a raw file download, **not** a
  Maven coordinate — useful for a manual `flatDir`-style local repo, but not
  what a real dependency declaration should point at. Note that this published
  0.1.0 asset is **arm64-v8a + armeabi-v7a only**; the x86_64 emulator slice
  lands with 0.1.1 on the next `v*` tag push.
- **Maven publish (issue #125)**: `genesisdb/build.gradle.kts`'s
  `publishing {}` block + `.github/workflows/release.yml`'s `android-publish`
  job publish `dev.genesisblock:genesisdb-android:0.1.1` to **GitHub
  Packages** (not Maven Central — no new account/GPG-signing setup needed,
  reuses the workflow's own `GITHUB_TOKEN`) on every `v*` tag push. As of this
  writing that job exists but hasn't run yet — the coordinate doesn't resolve
  until the next tag push triggers it. Once it has, consumers add:
  ```kotlin
  // settings.gradle.kts
  dependencyResolutionManagement {
      repositories {
          maven {
              url = uri("https://maven.pkg.github.com/Freshair129/GenesisBlock")
              credentials {
                  // GitHub Packages requires auth for EVERY read, even on a
                  // public repo — unlike Maven Central. A PAT with just
                  // `read:packages` scope is enough; it does not need to be
                  // yours specifically, any account with repo read access works.
                  username = "<your-github-username>"
                  password = "<a GitHub PAT with read:packages>"
              }
          }
      }
  }
  ```
  and then `implementation("dev.genesisblock:genesisdb-android:0.1.1")` as
  normal. Maven Central (fully anonymous resolution, no consumer PAT needed)
  remains a documented future option once `dev.genesisblock`'s Central Portal
  namespace is verified — see issue #125.

## Wire format gotcha

The JNI bridge serializes the *same* `serde`-derived Rust structs the REST
server uses (`NodeInput`, `NodeOutput`, ...), with **no** `rename_all`
attribute — so the JSON crossing the JNI boundary is **snake_case**
(`valid_from`, `query_vector`, ...), not the camelCase seen in the Node addon's
`index.d.ts` (that's a napi-rs-specific binding convention, not the wire
format). `Types.kt` carries an explicit `@SerialName` per field for this
reason — if you add a field, mirror the exact Rust field name in the
`@SerialName`, not the napi/TS name.
