// swift-tools-version:5.9
// GenesisBlockDB Swift SDK (MARK XVI Phase B-1). See docs/SPEC--MOBILE-SDK.md §B-1.
//
// IMPORTANT — this Package.swift is for LOCAL DEV + CI TESTING ONLY, not (yet)
// the shape of the published SPM package. It links `GenesisDB` directly
// against a locally-built `libgenesis_block_native.a` via `unsafeFlags`
// (SwiftPM only permits `unsafeFlags` in packages built directly, never in a
// package consumed as someone else's dependency — which is exactly the
// constraint we want right now, since there is no public release yet). Per
// the spec, the eventual published package swaps `GenesisDB`'s C dependency
// for a `.binaryTarget(name:, url:, checksum:)` pointing at a release
// `GenesisBlockDB.xcframework` asset — no `unsafeFlags`, safe to depend on
// from another project. That swap happens at the first real release; until
// then, building this package (whether for tests or the xcframework
// verification job) requires the Rust static lib already built locally —
// see README.md "Building".
import Foundation
import PackageDescription

// The directory containing the already-built `libgenesis_block_native.a`.
// CI sets `GENESISDB_RUST_LIB_DIR` explicitly per job (see
// .github/workflows/mobile-build.yml); the default matches what
// `cargo build --no-default-features --features "mobile ffi"` (deliberately
// NO --target — cargo then builds for the HOST's own default triple,
// whatever that is, sidestepping any Intel-vs-Apple-Silicon runner-arch
// guessing) produces when run from the repo root, for local Mac dev. This
// needs to be a HOST build, not the `aarch64-apple-ios`/`-sim` cross-compiled
// slices the xcframework uses — those can't execute on the build machine, so
// `GenesisDBTests` couldn't actually link-and-run against them.
let rustLibDir =
    ProcessInfo.processInfo.environment["GENESISDB_RUST_LIB_DIR"]
    ?? "../../target/debug"

let package = Package(
    name: "GenesisDB",
    platforms: [.macOS(.v13), .iOS(.v15)],
    products: [
        .library(name: "GenesisDBTypes", targets: ["GenesisDBTypes"]),
        .library(name: "GenesisDB", targets: ["GenesisDB"]),
    ],
    targets: [
        // Pure Swift wire-format types — zero C dependency, so
        // GenesisDBTypesTests can run without the Rust lib being built at
        // all (mirrors android-jvm-tests' no-native-lib property exactly).
        .target(name: "GenesisDBTypes"),
        .testTarget(name: "GenesisDBTypesTests", dependencies: ["GenesisDBTypes"]),

        // The C ABI (src/ffi.rs) as a system-library module. See
        // Sources/CGenesisDBFFI/module.modulemap for why the header isn't
        // committed here.
        .systemLibrary(name: "CGenesisDBFFI"),

        // The actor wrapper. Only this target (and its test target) requires
        // the Rust static lib to actually be present at build time.
        .target(
            name: "GenesisDB",
            dependencies: ["GenesisDBTypes", "CGenesisDBFFI"],
            linkerSettings: [
                .unsafeFlags(["-L\(rustLibDir)"]),
                .linkedLibrary("genesis_block_native"),
            ]
        ),
        .testTarget(name: "GenesisDBTests", dependencies: ["GenesisDB"]),
    ]
)
