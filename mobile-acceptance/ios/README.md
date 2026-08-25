# GenesisAcceptance — iOS on-device acceptance test

MARK XVI on-device acceptance ([issue #125](https://github.com/Freshair129/GenesisBlock/issues/125)
follow-up). This is a **genuinely independent, blank consumer** SPM package —
not a subdirectory of, or dependency on, [`../../ios/genesisdb`](../../ios/genesisdb)
(the actual SDK source).

## What this proves that `ios/genesisdb`'s own tests don't

`ios/genesisdb`'s `Package.swift` deliberately links a local host-arch build
of the Rust core (see its README, "Building" / "Prebuilt xcframework") —
that's the right call for *that* package, since it lets `GenesisDBTests`
actually execute during development, but it means `ios/genesisdb`'s CI never
exercises the thing a real external consumer actually does: fetch the
*published* `GenesisBlockDB.xcframework` release asset and link against it.

This package does exactly that:

```swift
.binaryTarget(
    name: "GenesisBlockDB",
    url: "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.0/GenesisBlockDB.xcframework.zip",
    checksum: "8359846a8e668770816e0d84940aead0a85812f5aa67f91e7c2ff8308d37bc72"
)
```

`RoundTripTests.swift` then calls the raw `genesisdb_*` C functions directly
(not through `ios/genesisdb`'s `GenesisDB` actor wrapper) — a real consumer
pointed only at the xcframework, with no SPM-registry entry for
`ios/genesisdb` itself (deliberately deferred, see issue #125), would have to
write the same kind of thin wrapper themselves.

## Why this can actually *run*, not just compile

The xcframework's `aarch64-apple-ios-sim` slice is cross-compiled for the
Simulator ABI — it can't execute as a plain host process, but it **can**
execute *inside* the iOS Simulator on an Apple Silicon macOS runner, since
that's a native arm64 execution environment. CI (`.github/workflows/mobile-build.yml`,
job `ios-acceptance-test`) runs:

```bash
xcodebuild test -scheme GenesisAcceptance -destination "id=<a real simulator udid>"
```

which builds this package for the Simulator destination and genuinely
executes `RoundTripTests` inside it — an `addNode`/`retrieveContext`/
`flush_index` round trip against the real published binary, not a local
build.

## Running locally

Requires a macOS/Xcode toolchain — cannot be exercised on the Windows dev
host (same carve-out as every other iOS piece in this repo):

```bash
cd mobile-acceptance/ios
xcodebuild test -scheme GenesisAcceptance -destination 'platform=iOS Simulator,name=<any available iPhone>'
```

## Keeping this in sync

The URL/checksum here are pinned to the `v0.2.0` release, same as
`react-native-genesisdb.podspec`'s `prepare_command` and `ios/README.md`'s
"Prebuilt xcframework" section — there's no automatic sync mechanism yet.
Bump all three together by hand whenever a new xcframework is published.
