---
version: "1.0.0"
created_at: "2026-06-29"
status: "phase-0-complete"
mark: "XVI"
complexity: "C-3"
doc_type: "spec"
scope: "new-surface"
---

# SPEC — GenesisBlock Mobile SDK (MARK XVI)

**Theme:** Bring GenesisBlockDB to mobile as a first-class embedded engine — local-first,
in-process, no remote server required. Delivers two parallel artifacts: a standalone mobile
app with graph + retriever UI (Level A), and a reusable SDK that any mobile developer can
embed in their own app (Level B). Level A ships first to validate the engine on real devices
before Level B extracts the SDK layer.

**Related docs:**
- Architecture map: [C4--GENESISDB-ARCHITECTURE.md](C4--GENESISDB-ARCHITECTURE.md)
- Master spec: [MASTER-SPEC--GENESIS-DB.md](MASTER-SPEC--GENESIS-DB.md)
- Obsidian plugin (existing thin-client reference): `obsidian-plugin/main.ts`
- WAL compaction (sandboxed path dependency): [ADR--PHASE-13-WAL-GROUP-COMMIT.md](adr/ADR--PHASE-13-WAL-GROUP-COMMIT.md)

---

## Motivation

The current mobile story is zero. The Obsidian plugin calls the Axum REST server over HTTP —
it requires a running server process, which is a non-starter on mobile. Every other surface
(Node NAPI, Python SDK, Go SDK, REST server) targets desktop or server.

**Target:** GenesisBlockDB runs embedded in a mobile process the same way SQLite does.
No network. No server. WAL + HNSW + GRL available in-process on iOS and Android.

**Comparators this closes:**
- SQLite / Realm / WatermelonDB — local relational/document on mobile
- Chroma (no official mobile SDK; clients call HTTP server)
- Qdrant (no embedded mobile; server only)

GenesisBlock would be the only embedded **hybrid semantic-graph** engine on mobile.

---

## Constraints

| Constraint | Detail |
|---|---|
| iOS sandbox | Each app can only read its own `Documents/` dir. No shared DB across apps. No background server process (iOS kills them). |
| Android sandbox | Similar; background services possible via `foreground-service` but complex. |
| `sysinfo` RSS probe | `sysinfo` uses `/proc/meminfo` and OS APIs that fail in mobile sandboxes. **Bench-only** — not used in `src/`, only by the bench harnesses; made optional and owned by the `bins` feature, so it never enters a `mobile` build (see 0-A). |
| `axum` + `tower-http` | REST server is not needed on mobile. Already behind `bins` feature — no change needed. |
| `rand` getrandom | Works on iOS/Android via OS entropy. No extra config. |
| HNSW arena size | Mobile RAM budget is ~300 MB–2 GB. Must expose `arena_capacity_mb` as a Tauri/FFI config param. |
| Tokio runtime | `tokio` full runtime works on iOS/Android. HNSW async indexing thread works as-is. |

---

## Phase 0 — Foundation

**Duration:** ~1 week  
**Goal:** The Rust core cross-compiles for mobile targets. No app, no UI — just a clean
binary that links.  
**All subsequent phases depend on this.**

### 0-A: `mobile` Cargo feature — **DONE (verified)**

> **Correction (2026-06-29):** The original plan assumed `sysinfo` was used inside
> `src/lib.rs` and would need `#[cfg(feature = "mobile")]` gating plus a `get_diagnostics`
> shim. A grep proved this wrong: **`sysinfo` is not referenced anywhere in `src/`.** It is
> used *only* by three bench harnesses (`benches/edge_interning_audit.rs`,
> `benches/graph_bench.rs`, `benches/vbench_genesis.rs`), all of which are pulled in through
> the `bins` feature. So no `src/lib.rs` change is needed — the fix is purely in `Cargo.toml`
> dependency wiring.

The actual approach: make `sysinfo` an **optional** dependency owned by the `bins` feature
(the benches pull it in), and add the `mobile` and `ffi` features. Because nothing in the
storage core touches `sysinfo`, a `--features mobile` build simply never compiles it.

```toml
[features]
default = ["napi-bindings"]
napi-bindings = ["dep:napi", "dep:napi-derive"]
# `sysinfo` is only used by the bench harnesses (RSS/mem probes), so it is pulled
# in here rather than as a hard dependency — that keeps it out of mobile builds.
bins = ["dep:sysinfo"]
# `mobile` builds the storage core for iOS/Android targets: no napi, no bins, no sysinfo.
mobile = []
# `ffi` exposes the C ABI layer (src/ffi.rs). Composes with `mobile`: --features "mobile ffi".
ffi = []

[dependencies]
# was a hard dependency; now optional and only enabled by `bins`
sysinfo = { version = "0.30", optional = true }
```

No `src/lib.rs` edit is required — there is no `sysinfo` import or `get_diagnostics` probe in
the core to gate.

Build command for this phase:
```bash
cargo build --no-default-features --features mobile
```

**Status:** DONE. Verified locally — `cargo build --no-default-features --features mobile`
exits 0 with no `sysinfo` compiled in.

### 0-B: Cross-compile probe — **DONE (CI-validated)**

**Status:** DONE 2026-06-29. `.github/workflows/mobile-build.yml` builds the core for
`aarch64-apple-ios` + `aarch64-apple-ios-sim` (macOS runner) and `aarch64-linux-android` +
`armv7-linux-androideabi` (Linux runner, cargo-ndk). All green on PR #37. This CI is the
**only** place iOS/Android linking is exercised — the dev host is Windows (no macOS/Xcode,
no local NDK), so cross-compile cannot be reproduced locally.

```bash
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi

# iOS device
cargo build --no-default-features --features mobile --target aarch64-apple-ios

# iOS Simulator (Apple Silicon Mac)
cargo build --no-default-features --features mobile --target aarch64-apple-ios-sim

# Android arm64
cargo build --no-default-features --features mobile --target aarch64-linux-android \
  --config target.aarch64-linux-android.linker="<ndk-path>/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
```

Expected failures to fix: any transitive dep that calls `sysinfo`, any `std::fs` path
that assumes a non-sandboxed layout (there should be none — `Storage::open` already takes
an explicit path), any dep that uses `getrandom` without mobile backend.

**Acceptance:** `cargo build --target aarch64-apple-ios` exits 0 with warnings only.

### 0-C: C FFI layer (`src/ffi.rs`) — **DONE**

**Status:** DONE 2026-06-29 (shipped in PR #37). Implemented as 8 `#[no_mangle]` symbols
(`genesisdb_open`/`close`/`add_node`/`search`/`execute_hql`/`retrieve_context`/
`flush_index`/`free_string`), gated `#[cfg(feature = "ffi")]`, every entry point wrapped in
`catch_unwind`. The handle boxes `Arc<Storage>` and calls the **synchronous** `Storage`
methods directly (the tokio/`spawn_blocking` offload lives only in the napi-only
`GenesisDatabase` wrapper, so no runtime is pulled into the mobile binary).

Required for Phase B (SDK). Tauri (Phase A) calls Rust directly, but the iOS xcframework
and Android JNI both need a stable C ABI.

```rust
// src/ffi.rs  — feature-gated: #[cfg(any(feature = "mobile", feature = "ffi"))]
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Open or create a GenesisDB at `path`. Returns an opaque handle.
#[no_mangle]
pub extern "C" fn genesisdb_open(path: *const c_char) -> *mut GenesisHandle { ... }

/// Close and free the handle.
#[no_mangle]
pub extern "C" fn genesisdb_close(handle: *mut GenesisHandle) { ... }

/// Add a node. `json_input` is a NodeInput JSON string.
/// Returns a JSON string (NodeOutput) — caller must free with genesisdb_free_string.
#[no_mangle]
pub extern "C" fn genesisdb_add_node(handle: *mut GenesisHandle, json_input: *const c_char) -> *const c_char { ... }

/// Execute an HQL query string. Returns JSON result.
#[no_mangle]
pub extern "C" fn genesisdb_execute_hql(handle: *mut GenesisHandle, hql: *const c_char) -> *const c_char { ... }

/// Free a string returned by genesisdb_*.
#[no_mangle]
pub extern "C" fn genesisdb_free_string(s: *const c_char) { ... }

// Also: genesisdb_search, genesisdb_retrieve_context, genesisdb_flush_index,
//        genesisdb_get_graph_snapshot (returns nodes+edges JSON for graph view)
```

The JSON-in / JSON-out contract mirrors the existing REST API body shapes, so NAPI and
REST tests serve as implicit contract tests for the FFI layer.

### 0-D: WAL path injection — no core change needed (verified)

The constructor already takes the DB root as an explicit caller-supplied path, so the engine
never assumes a fixed or non-sandboxed location. Verified against the code:

```rust
// src/lib.rs:1418
pub fn open(opts: OpenOptions) -> Result<Self> {
    let root = PathBuf::from(opts.path.clone());   // line 1419 — DB root from caller
    if !root.exists() {
        fs::create_dir_all(&root).ok();            // creates the dir if missing
    }
    ...
}

// src/lib.rs:78 — the path arrives as a field of OpenOptions, not a bare &str
pub struct OpenOptions {
    pub path: String,
    pub page_cache_mb: Option<u32>,
    pub read_only: Option<bool>,
    pub vector_dim: Option<u32>,
}
```

The WAL, snapshot (`state.json`), and per-collection `vec_*.bin`/`meta_*.bin` files are all
written under this `root`. **The mobile caller's only responsibility is to pass the
OS-provided sandboxed directory as `OpenOptions.path`** — every platform exposes one:

| Platform | API | Resolved DB root |
|---|---|---|
| iOS | `NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)` (a.k.a. `NSDocumentDirectory`) | `.../Documents/genesisdb/` |
| Android | `context.getFilesDir()` | `.../files/genesisdb/` |
| Tauri | `app.path().app_data_dir()` | platform-appropriate app-data dir, then `/genesisdb/` |

Convention: append a `genesisdb/` subdirectory to the platform sandbox root so the engine's
files are namespaced and easy to back up / clear as a unit. `Storage::open` will
`create_dir_all` it on first launch (see line 1421 above).

**Conclusion:** no engine change is required for path injection — this is purely a convention
documented for SDK consumers. (The only nuance vs. the original wording: the path is supplied
as `OpenOptions.path: String`, not a positional `&str` argument.)

**Phase 0 definition of done:**
- [ ] `cargo build --no-default-features --features mobile --target aarch64-apple-ios` exits 0
- [ ] `cargo build --no-default-features --features mobile --target aarch64-linux-android` exits 0
- [ ] `src/ffi.rs` compiles and exports the 7 core symbols
- [ ] `cargo test --no-default-features --features mobile` still passes all integration tests

---

## Phase A — GenesisBlock Mobile App

**Duration:** ~3 weeks  
**Goal:** A single Tauri v2 app (iOS + Android) with an embedded GenesisBlockDB, a
sigma.js graph view, and a GRL retriever panel.  
**Validates:** engine correctness on real mobile hardware before SDK extraction.

### Directory layout

```
genesisblock-mobile/
├── src-tauri/
│   ├── Cargo.toml          # dep: genesis-block-native path=".." no-default-features features=["mobile"]
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs         # Tauri app entry; init Storage with app_data_dir()
│   │   └── commands.rs     # Tauri #[command] handlers
│   ├── gen/
│   │   ├── apple/          # Xcode project (generated by tauri ios init)
│   │   └── android/        # Gradle project (generated by tauri android init)
│   ├── icons/
│   └── tauri.conf.json
├── src/                    # WebView frontend (React + TypeScript)
│   ├── components/
│   │   ├── GraphView.tsx   # sigma.js WebGL graph
│   │   └── RetrieverPanel.tsx
│   ├── hooks/
│   │   ├── useGraph.ts     # polls get_graph_snapshot every 2s
│   │   └── useRetriever.ts # calls execute_hql CONTEXT
│   ├── lib/
│   │   └── tauri.ts        # invoke() wrappers with TypeScript types
│   ├── App.tsx
│   └── main.ts
├── package.json
└── vite.config.ts
```

### Tauri commands (`src-tauri/src/commands.rs`)

These map 1-to-1 with existing NAPI methods — same input/output types, different surface:

```rust
#[tauri::command]
async fn add_node(state: State<'_, AppState>, input: NodeInput) -> Result<NodeOutput, String>

#[tauri::command]
async fn search(state: State<'_, AppState>, input: HybridSearchInput) -> Result<Vec<NodeOutput>, String>

#[tauri::command]
async fn execute_hql(state: State<'_, AppState>, query: String) -> Result<serde_json::Value, String>

#[tauri::command]
async fn retrieve_context(state: State<'_, AppState>, input: RetrieveContextInput) -> Result<ContextResult, String>

#[tauri::command]
async fn get_graph_snapshot(state: State<'_, AppState>) -> Result<GraphSnapshot, String>
// Returns: { nodes: [{id, label, tier, x?, y?}], edges: [{from, to, relation, valid}] }

#[tauri::command]
async fn flush_index(state: State<'_, AppState>) -> Result<(), String>

#[tauri::command]
async fn get_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticsOutput, String>
// sysinfo is bench-only and absent from `mobile` builds; if a diagnostics command is
// surfaced on mobile it must source RSS from an OS-appropriate API or return engine-level
// counters (e.g. index_lag, node/edge counts) rather than a sysinfo probe.
```

### Graph View (sigma.js)

```typescript
// src/components/GraphView.tsx
import { Sigma } from "sigma";
import { DirectedGraph } from "graphology";

// Node visual encoding
const tierColor = {
  MASTER:   "#e3b341",  // gold
  EXPERT:   "#388bfd",  // blue
  STANDARD: "#3fb950",  // green
  OBSERVER: "#8b949e",  // grey
};

// Data source: get_graph_snapshot() polled every 2s
// Only renders edges where valid_to == null (current view)
// Tap node → set selectedNodeId → RetrieverPanel fires CONTEXT query
// Pinch-zoom + pan via Sigma built-in camera controls (works on touch)
```

**Why sigma.js:** WebGL-backed, handles 5k+ nodes at 60 fps on mobile, built-in touch
support, smaller bundle than vis-network.

### Retriever Panel

```typescript
// src/components/RetrieverPanel.tsx
// Input: free-text query OR selected node id from graph tap
// Command: CONTEXT "<query>" TIER H0 H1 H2 H3 LIMIT 20
// Output: tier cards H0 (exact match) → H5 (broad context)
// Side effect: highlight matching nodes on graph (sigma setHighlight)
// Toggle: CRDT sync on/off (calls Tauri command toggle_sync)
```

### Build commands

```bash
cd genesisblock-mobile

# Dev (desktop WebView for fast iteration)
npm run tauri dev

# iOS
npm run tauri ios build
# → outputs: gen/apple/build/Build/Products/Release-iphoneos/GenesisBlock.app

# Android
npm run tauri android build
# → outputs: gen/android/app/build/outputs/apk/release/app-release.apk
```

**Phase A definition of done:**
- [ ] App installs and launches on a physical iOS device (iPhone, not Simulator only)
- [ ] App installs and launches on a physical Android device (arm64)
- [ ] `add_node` → node appears in graph view within 3s (async HNSW indexing)
- [ ] CONTEXT query returns H0–H5 tiers in retriever panel
- [ ] 1000 nodes render at ≥30 fps on graph view
- [ ] WAL survives app restart (data persists across launches)
- [ ] CRDT sync successfully exchanges events with a desktop GenesisDB instance

---

## Phase B — SDK for Other Apps

**Duration:** ~6 weeks  
**Goal:** Any mobile developer can embed GenesisBlockDB in their app without writing Rust.
The SDK is extracted from the proven Phase A core.  
**Input:** Phase A shipped and validated on real hardware.

> **Foundations landed (2026-06-29, branch `feat/mxvi-phase-b-foundations`).** The
> host/CI-verifiable Rust layer that both B-1 and B-2 sit on top of is in place — the
> remaining B-1/B-2/B-3 work is platform glue (Swift/Kotlin/RN wrappers + xcframework/.aar
> assembly) that can only be built/validated on macOS/NDK/devices via CI:
> - **C header** — `include/genesisdb.h` generated from `src/ffi.rs` by `cbindgen`
>   (`cbindgen.toml` + `scripts/gen-header.sh`); the xcframework's `-headers` input. Committed
>   and verified fresh by the `c-header-freshness` CI gate.
> - **JNI bridge** — `src/jni.rs` behind the new `android-jni` feature (pulls the pure-Rust
>   `jni` crate), exporting `Java_dev_genesisblock_GenesisDB_native*` over the sync `Storage`
>   core, panic-safe and JSON-in/out exactly like `src/ffi.rs`.
> - **crate-type** — added `staticlib` (the iOS `.a`) alongside `cdylib` (Android `.so`).
> - **CI** — `mobile-build.yml` now builds the real SDK feature sets
>   (`mobile ffi` for iOS, `mobile ffi android-jni` for Android), verifies the `.a`/`.so`
>   slices are produced, runs the integration tests under `mobile ffi android-jni` on the
>   host, and gates header freshness. (Previously it built only `--features mobile`, so the
>   FFI/JNI surface was never cross-compiled.)

### B-1: iOS xcframework + Swift wrapper (~2 weeks)

```bash
# Build static libs for both slices
cargo build --release --no-default-features --features "mobile ffi" --target aarch64-apple-ios
cargo build --release --no-default-features --features "mobile ffi" --target aarch64-apple-ios-sim

# Combine into xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libgenesis_block_native.a \
  -headers include/ \
  -library target/aarch64-apple-ios-sim/release/libgenesis_block_native.a \
  -headers include/ \
  -output GenesisBlockDB.xcframework
```

Swift wrapper (`GenesisDB.swift`):
```swift
public actor GenesisDB {
    private let handle: OpaquePointer

    public init(path: URL) throws { ... }

    public func addNode(_ input: NodeInput) async throws -> NodeOutput { ... }
    public func executeHQL(_ query: String) async throws -> HQLResult { ... }
    public func retrieveContext(_ input: ContextInput) async throws -> ContextResult { ... }
    public func search(_ input: SearchInput) async throws -> [NodeOutput] { ... }
}
```

Distribution: Swift Package Manager via `Package.swift` + binary target pointing to the
xcframework. Release CI uploads the xcframework as a GitHub release asset; `Package.swift`
references the release URL + checksum.

> **B-2 landed (2026-07-03).** `android/genesisdb/` is a real Gradle library
> module: `Types.kt` (data classes with explicit `@SerialName` snake_case wire
> mapping — the FFI/JNI JSON contract is the engine's raw un-renamed
> `serde_json` output, NOT the camelCase in `index.d.ts`, which is a
> napi-rs-only convention) and `GenesisDB.kt` (coroutine wrapper over the 7
> `src/jni.rs` symbols). `WireFormatTest.kt` proves the wire contract on pure
> JVM (no native lib, no NDK). CI (`mobile-build.yml`): `android-jvm-tests`
> runs those tests on every PR; `android-aar` assembles a real `.aar` on top
> of `android-build`'s cargo-ndk `.so` output and uploads it as a build
> artifact. `scripts/gen-android-jnilibs.sh` mirrors the CI staging step for
> local dev (still host-only — no NDK on the Windows dev box). Not yet done:
> publishing the `.aar` to Maven Central/GitHub Packages (still `0.1.0`,
> unpublished) and the on-device/Gradle-project acceptance checks below.

### B-2: Android .aar + Kotlin wrapper (~2 weeks)

JNI bridge (`src/jni.rs`):
```rust
#[cfg(feature = "android-jni")]
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeOpen(
    env: JNIEnv, _: JClass, path: JString
) -> jlong { ... }

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeExecuteHQL(
    env: JNIEnv, _: JObject, handle: jlong, query: JString
) -> jstring { ... }
```

Kotlin wrapper (`GenesisDB.kt`):
```kotlin
class GenesisDB(path: File) : Closeable {
    private val handle: Long = nativeOpen(path.absolutePath)

    suspend fun addNode(input: NodeInput): NodeOutput = withContext(Dispatchers.IO) { ... }
    suspend fun executeHQL(query: String): HQLResult = withContext(Dispatchers.IO) { ... }
    suspend fun retrieveContext(input: ContextInput): ContextResult = withContext(Dispatchers.IO) { ... }

    override fun close() = nativeClose(handle)

    companion object {
        init { System.loadLibrary("genesis_block_native") }
        private external fun nativeOpen(path: String): Long
        private external fun nativeClose(handle: Long)
        private external fun nativeExecuteHQL(handle: Long, query: String): String
    }
}
```

Distribution: `.aar` published to Maven Central or GitHub Packages.

```gradle
dependencies {
    implementation("dev.genesisblock:genesisdb-android:0.1.0")
}
```

> **B-3 landed for Android (2026-07-03).** `react-native-genesisdb/` is a real
> npm package: `src/types.ts` (snake_case wire types, matching the
> `genesisdb-python`/`genesisdb-go` precedent — deliberately NOT translated to
> camelCase, since a generic deep-recasing layer would corrupt caller keys
> inside the opaque `props` field), `src/index.ts` (the public `GenesisDB`
> class, JSON pass-through only, no marshalling logic to get wrong), and
> `android/` (`GenesisDbModule.kt` bridging RN's Promise-based API to Phase
> B-2's `dev.genesisblock.GenesisDB`, using a small opaque `dbId` int instead
> of the raw native pointer to avoid JS-number precision loss on the bridge).
> `src/__tests__/index.test.ts` covers the pass-through layer under plain
> Jest (`rn-genesisdb-tests` CI job) — no RN runtime needed. **iOS is a stub**
> (`ios/GenesisDbModule.swift` + `.m` + podspec): `pod install` and
> autolinking succeed, but every method rejects with
> `GENESISDB_IOS_NOT_IMPLEMENTED` pending B-1. Not yet done: publishing to
> npm, and building/testing the native modules inside a real RN host app
> (out of scope for this monorepo's CI — same host-only carve-out as B-1/B-2).

### B-3: React Native package (~2 weeks, parallel with B-1/B-2)

```
react-native-genesisdb/
├── ios/
│   └── GenesisDbModule.swift   # RCT_EXPORT_MODULE; calls Swift wrapper
├── android/
│   └── GenesisDbModule.kt      # ReactContextBaseJavaModule; calls Kotlin wrapper
├── src/
│   └── index.ts                # NativeModules.GenesisDb + TS types ≈ index.d.ts
└── package.json
```

TypeScript API intentionally mirrors `index.d.ts` (the existing NAPI types) so knowledge
of one surface transfers to the other.

Distribution:
```bash
npm install react-native-genesisdb
```

### B-4: Flutter plugin (deferred — demand-dependent)

Uses `flutter_rust_bridge` to auto-generate Dart bindings from `src/ffi.rs`. Not in scope
for the initial SDK release; added when there is demonstrated user demand.

### Phase B definition of done:

**iOS SDK:**
- [ ] `GenesisBlockDB.xcframework` builds for `aarch64-apple-ios` + `aarch64-apple-ios-sim`
- [ ] Swift Package Manager `import GenesisBlockDB` works in a blank Xcode project
- [ ] `addNode` + `retrieveContext` round-trip in a Swift test target

**Android SDK:**
- [ ] `.aar` installs via Gradle in a blank Android Studio project
- [ ] `GenesisDB(filesDir).executeHQL(...)` runs on a physical arm64 device
- [ ] JNI `UnsatisfiedLinkError` does not occur at runtime

**React Native package:**
- [ ] `npm install react-native-genesisdb` + `npx pod-install` works on iOS
- [ ] `npm install react-native-genesisdb` + Gradle sync works on Android
- [ ] TypeScript types have zero `any` — full inference on all public methods

---

## Dependency on existing engine features

| Mobile feature | Existing engine capability used |
|---|---|
| Graph view nodes | `get_all_nodes` or HQL `MATCH * LIMIT n` |
| Graph view edges | `get_neighbors` / `out_idx`+`in_idx` traversal |
| Retriever panel | `Storage::retrieve_context` → HQL `CONTEXT` |
| In-process search | `Storage::hybrid_search` |
| Data persistence | WAL + `save_state` (unchanged) |
| Sync with desktop | CRDT `reconcile_state` + gossip (existing, Phase A toggle only) |
| Governance on mobile | Enforced by engine — MASTER tier guard unchanged |

---

## Versioning model — is mobile shared with desktop, or counted separately?

**Two layers, two answers.**

**1. The engine core is ONE version, shared.** Desktop and mobile are the *same crate*
(`genesis-block-native`) compiled with different feature flags — not separate codebases:

| Target | Build |
|---|---|
| Desktop — Node addon | `napi build` (default `napi-bindings`) → cdylib |
| Desktop — REST server | `--no-default-features --features bins` → rlib |
| Mobile — iOS / Android | `--no-default-features --features "mobile ffi"` → staticlib |

All three come from one `Cargo.toml` with one `version`. They **cannot diverge** — there is
no separate "mobile version" of the engine. As of this writing every target is `0.2.0`. A
bug fix or schema change bumps the single engine version and all surfaces inherit it. The
on-disk format is tracked orthogonally by `SCHEMA_VERSION` (`src/lib.rs`), also shared.

**2. Distribution packages are counted SEPARATELY, pinned to a minimum engine version.**
`modules.json` is the manifest: the `engine` block holds the one shared engine version, and
each consumer *surface* carries its own `version` + a `minEngineVersion`. This already exists
for the non-mobile surfaces (Python SDK `0.1.0`, Go SDK `0.1.0`, MCP `0.1.0-beta.1`, etc.) —
each ships on its own cadence but declares the oldest engine it works against.

The future mobile artifacts follow the same pattern — each becomes a new surface entry:

| Surface (Phase A/B) | Own `version` | `minEngineVersion` |
|---|---|---|
| `genesisblock-mobile` (Tauri app) | independent | the engine it bundles |
| iOS `GenesisBlockDB.xcframework` | independent | the engine it wraps |
| Android `genesisdb-android` (.aar) | independent | the engine it wraps |
| `react-native-genesisdb` | independent | the engine it wraps |

**Policy (0.x / beta):** keep each mobile package's `version` **coupled** to the engine
version (ship `0.2.x` packages against a `0.2.x` engine) for simplicity — one number to
reason about. **Decouple at 1.0**, once the engine API and the SDK ergonomics stabilise
independently. The `scripts/version.mjs` SSOT + the `version-consistency` CI gate enforce
that the engine value stays identical across `Cargo.toml` / `package.json` / `modules.json`;
per-surface versions are managed in `modules.json` and are intentionally *not* gated.

**TL;DR:** the *engine* is one version, shared by desktop and mobile (same crate, different
features). The *shipping packages* are versioned separately but each pins a `minEngineVersion`.

---

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| `hnsw_rs` cross-compile fails | Low — pure Rust | Pin to known good version; vendor if needed |
| iOS background kill during HNSW index flush | Medium | Call `flush_index()` on `applicationWillResignActive` |
| Android 32-bit armeabi-v7a RAM limits | Medium | Cap `arena_capacity_mb` per ABI at build time |
| CRDT gossip UDP blocked on mobile networks | High | UDP gossip is optional; fall back to HTTP push for sync |
| App Store rejection (background network) | Low | Sync is user-initiated; no background server socket |
| WAL grows unbounded on device | Medium | WAL compaction (PR #28) already shipped — trigger on app foreground |

---

## Open questions

1. **Graph view data source:** poll `get_graph_snapshot()` every N seconds, or stream via
   Tauri events? Polling is simpler; events add latency for large graphs.
2. **Embedding on mobile:** who generates the vector embeddings? On-device (Core ML / ONNX
   Runtime) or user-supplied float arrays? Phase A defers this — `NodeInput.embedding` is
   optional and the retriever panel works on graph structure without vectors.
3. **SDK versioning:** ~~pin SDK version to engine semver or decouple?~~ **Resolved** — see
   the [Versioning model](#versioning-model--is-mobile-shared-with-desktop-or-counted-separately)
   section: engine is one shared version; packages are separate but pinned to a
   `minEngineVersion`; coupled during 0.x, decoupled at 1.0.
4. **App Store distribution of xcframework:** binary targets in SPM require a checksum and
   a public release URL. Private distribution via XCFramework zip + local `Package.swift`
   is also viable for enterprise customers.
