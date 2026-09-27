---
status: accepted
owner: GenesisBlockDB Engineering
date: 2026-09-04
issue: 166
---

# ADR — iOS Swift Package Distribution Boundary

## Context

GenesisBlockDB already publishes a CI-built `GenesisBlockDB.xcframework.zip` release artifact and has an external acceptance fixture under `mobile-acceptance/ios/` that consumes the binary with SwiftPM `.binaryTarget(url:checksum:)`.

However, the repository does **not** currently expose a root-level public Swift package that a consumer can add by repository URL.

A tempting shortcut is to add a root `Package.swift` today and point it at the existing `v0.2.0` xcframework asset. That would create a version-integrity defect: a newer repository/package tag could resolve an older engine binary while presenting itself as the newer package version.

The source Swift package under `ios/genesisdb/` also has a different responsibility: it links a host-built Rust static library so its own tests can execute on the build host. Replacing that development path with the cross-compiled xcframework would weaken internal test coverage.

## Decision

**Keep the GitHub Release xcframework as the canonical native iOS binary distribution for now. Do not add a root public SPM package that is pinned to a stale binary release.**

The existing source-development Swift package and the external binary-consumer acceptance fixture remain separate on purpose.

## Required gate before public root SPM

A public SwiftPM consumer path may be introduced only when release automation guarantees all of the following for the same version:

1. build `GenesisBlockDB.xcframework` from the exact release commit;
2. produce the distributable zip in a SwiftPM-compatible archive format;
3. compute the final SHA-256/SwiftPM checksum before the package version is released;
4. place that URL + checksum in the public package manifest without pointing a new package version at an old engine binary;
5. publish/tag using a version policy that is unambiguous between engine and Swift SDK versions;
6. run an external consumer acceptance test against the published artifact in an iOS Simulator.

## Acceptable future designs

### Option A — coordinated monorepo release

Keep SwiftPM at the repository root, but change release sequencing so the xcframework artifact/checksum is produced and pinned before the semver tag that consumers resolve.

### Option B — dedicated Swift package repository

Publish a small Swift package repository whose versions track the iOS SDK independently and whose manifest references immutable GenesisBlockDB release assets.

This is preferable if the Swift SDK needs a version cadence independent from the engine monorepo.

## Acceptance criteria

- [ ] consumer can add a public repository URL in Xcode/SwiftPM with no monorepo checkout;
- [ ] package product exposes the GenesisBlockDB binary/module cleanly;
- [ ] package version and embedded engine artifact version are explicitly compatible;
- [ ] checksum is generated from the exact published zip;
- [ ] clean external Simulator round trip (`open` → `addNode` → read/retrieve) passes;
- [ ] release CI fails if manifest URL/checksum and release artifact drift;
- [ ] root/source SDK tests remain capable of exercising host-native builds where required.

## Consequences

### Positive

- avoids shipping a misleading SPM package whose semantic version does not match its binary;
- preserves the already-working xcframework consumer path;
- keeps internal source SDK tests independent from external binary distribution tests.

### Negative

- native Swift consumers still use the release xcframework/manual binary integration until the coordinated SPM release path is completed;
- adding the repository directly as a Swift package is intentionally not advertised yet.

## Rejected shortcut

A root `Package.swift` that simply points every future package version at the `v0.2.0` xcframework is rejected. It would improve installation ergonomics by silently breaking version truth, which is a bad trade.
