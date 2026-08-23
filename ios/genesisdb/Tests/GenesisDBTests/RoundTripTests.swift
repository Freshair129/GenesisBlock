// Real, executed round trip against the compiled engine — the Phase B DoD
// item "addNode + retrieveContext round-trip in a Swift test target"
// (docs/SPEC--MOBILE-SDK.md §B-1). Unlike WireFormatTests (zero C
// dependency), this target links `libgenesis_block_native.a` built for the
// HOST's own architecture (see Package.swift / README "Building") and calls
// the real C ABI end to end: open -> addNode -> retrieveContext -> close,
// against a fresh temp-directory database per test.
//
// This runs on macOS (the CI/dev-machine architecture), not an iOS
// device/simulator — but the code under test (GenesisDB.swift, the C
// interop, Codable (de)serialization) has no iOS-simulator-specific surface,
// so a passing run here is strong evidence the same code works on-device.
// Literal on-device/Xcode-project verification remains open — the same
// host-only carve-out already used for B-2's on-device Gradle check and
// B-3's real RN host-app testing.

import XCTest

@testable import GenesisDB
@testable import GenesisDBTypes

final class RoundTripTests: XCTestCase {
    private var tempDir: URL!

    override func setUpWithError() throws {
        tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("genesisdb-roundtrip-\(UUID().uuidString)")
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: tempDir)
    }

    func testAddNodeAndRetrieveContextRoundTrip() async throws {
        let db = try GenesisDB.open(path: tempDir)

        let added = try await db.addNode(
            NodeInput(id: "ctx_root", labels: ["Doc"], props: .object(["title": .string("hello")]))
        )
        XCTAssertEqual(added.id, "ctx_root")
        XCTAssertEqual(added.props["title"]?.stringValue, "hello")

        let pkg = try await db.retrieveContext(targetId: "ctx_root", tier: "H1")
        XCTAssertTrue(
            pkg.nodes.contains { $0.id == "ctx_root" },
            "expected ctx_root among retrieved nodes: \(pkg.nodes.map(\.id))"
        )
        // H1 = 1 hop (ScalingTier); an isolated root node exhausts its
        // reachable subgraph at depth 0, so the ceiling is never hit.
        XCTAssertEqual(pkg.coverage.hopsRequested, 1)
        XCTAssertFalse(pkg.coverage.ceilingHit)

        await db.close()
    }

    func testAddNodeWithEmbeddingIsSearchable() async throws {
        let db = try GenesisDB.open(path: tempDir)

        // `genesisdb_open` (src/ffi.rs) always opens with `vector_dim: None`,
        // and `Storage::open` resolves that to a **1536**-dim `default`
        // collection (`opts.vector_dim.unwrap_or(1536)`), not auto-detected
        // from the first embedding — a smaller vector is a dim mismatch, not
        // silently accepted. `queryVector`/`embedding` below are therefore
        // full 1536-length vectors, not a short hand-typed one.
        var embedding = [Double](repeating: 0.0, count: 1536)
        embedding[0] = 1.0
        _ = try await db.addNode(
            NodeInput(id: "vec1", labels: ["Doc"], embedding: embedding)
        )
        // Async HNSW indexing (ADR--GENESISDB-ASYNC-INDEXING): a just-staged
        // vector is only *eventually* searchable, so flush before asserting.
        try await db.flushIndex()

        let hits = try await db.search(
            HybridSearchInput(queryVector: embedding, k: 1, alpha: 0.0)
        )
        XCTAssertEqual(hits.first?.node.id, "vec1")

        await db.close()
    }

    func testCloseThenCallThrows() async throws {
        let db = try GenesisDB.open(path: tempDir)
        await db.close()
        do {
            _ = try await db.addNode(NodeInput(labels: ["Doc"]))
            XCTFail("expected addNode on a closed handle to throw")
        } catch is GenesisDBError {
            // expected
        }
    }
}
