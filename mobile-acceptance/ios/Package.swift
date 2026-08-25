// swift-tools-version:5.7
// GenesisAcceptance — MARK XVI on-device acceptance (issue #125 follow-up).
//
// This is a genuinely independent, "blank consumer" SPM package: it does NOT
// depend on ../../ios/genesisdb (the source SDK) or its local host-arch
// build. It depends ONLY on the already-published v0.2.0 GitHub Release
// asset via `.binaryTarget(url:, checksum:)` — the exact distribution
// mechanism a real external consumer would use, which `ios/genesisdb`'s own
// Package.swift deliberately does NOT exercise (see that package's README
// for why: swapping to binaryTarget there would break its own test
// executability, since the xcframework's slices can't run on the build
// host — but the *simulator* slice CAN run inside the iOS Simulator, which
// is exactly what this package's test target proves).
import PackageDescription

let package = Package(
    name: "GenesisAcceptance",
    platforms: [.iOS(.v13)],
    targets: [
        // Pinned to the v0.2.0 release (docs/... and ios/README.md "Prebuilt
        // xcframework" document the same URL/checksum). Bump both together
        // by hand whenever a new xcframework is published — there is no
        // automatic sync yet, same caveat as react-native-genesisdb's
        // podspec.
        .binaryTarget(
            name: "GenesisBlockDB",
            url: "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.0/GenesisBlockDB.xcframework.zip",
            checksum: "8359846a8e668770816e0d84940aead0a85812f5aa67f91e7c2ff8308d37bc72"
        ),
        .testTarget(
            name: "GenesisAcceptanceTests",
            dependencies: ["GenesisBlockDB"]
        ),
    ]
)
