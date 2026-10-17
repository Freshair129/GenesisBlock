// swift-tools-version:5.7
// GenesisAcceptance — MARK XVI on-device acceptance (issue #125 follow-up).
//
// This is a genuinely independent, "blank consumer" SPM package: it does NOT
// depend on ../../ios/genesisdb (the source SDK) or its local host-arch
// build. It defaults to the published v0.2.0 GitHub Release asset via
// `.binaryTarget(url:, checksum:)` — the exact distribution mechanism a real
// external consumer would use. Release CI overrides the URL/checksum to test
// the newly published tag, or uses a local path for a non-publishing dry run.
// `ios/genesisdb`'s own Package.swift deliberately does NOT exercise this
// binary target (see that package's README for why: swapping to a binary
// target there would break its host-arch test executability, since the
// xcframework's slices can't run on the build host — but the *simulator* slice
// CAN run inside the iOS Simulator, which is exactly what this test proves).
import Foundation
import PackageDescription

let environment = ProcessInfo.processInfo.environment
let releaseURL = environment["GENESISDB_XCFRAMEWORK_URL"]
let releaseChecksum = environment["GENESISDB_XCFRAMEWORK_CHECKSUM"]
let localPath = environment["GENESISDB_XCFRAMEWORK_PATH"]

precondition(
    (releaseURL == nil) == (releaseChecksum == nil),
    "Set GENESISDB_XCFRAMEWORK_URL and GENESISDB_XCFRAMEWORK_CHECKSUM together."
)
precondition(
    localPath == nil || (releaseURL == nil && releaseChecksum == nil),
    "Set GENESISDB_XCFRAMEWORK_PATH or the release URL/checksum pair, not both."
)

let xcframeworkTarget: Target
if let localPath = localPath {
    xcframeworkTarget = .binaryTarget(
        name: "GenesisBlockDB",
        path: localPath
    )
} else if let releaseURL = releaseURL, let releaseChecksum = releaseChecksum {
    xcframeworkTarget = .binaryTarget(
        name: "GenesisBlockDB",
        url: releaseURL,
        checksum: releaseChecksum
    )
} else {
    // Default stays pinned to the last explicitly reviewed release for
    // ordinary development runs. Release CI overrides both values to test the
    // artifact built for the exact tag being published.
    xcframeworkTarget = .binaryTarget(
        name: "GenesisBlockDB",
        url: "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.0/GenesisBlockDB.xcframework.zip",
        checksum: "607df0d82d68550a20927ae171928ad1decd7253fb647da450dec87deea1c26d"
    )
}

let package = Package(
    name: "GenesisAcceptance",
    platforms: [.iOS(.v13)],
    targets: [
        xcframeworkTarget,
        .testTarget(
            name: "GenesisAcceptanceTests",
            dependencies: ["GenesisBlockDB"]
        ),
    ]
)
