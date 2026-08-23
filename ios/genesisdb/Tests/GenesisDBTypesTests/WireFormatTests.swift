// Pure-Swift tests for the Swift <-> engine JSON wire contract. These load no
// native library and make no C call — they only prove that `Types.swift`'s
// `CodingKeys` produce/parse the exact snake_case shape the engine's
// `serde_json` (un-renamed) structs use on the wire (see the "Wire format
// gotcha" note in ios/README.md, and android/genesisdb's WireFormatTest.kt,
// which this file mirrors test-for-test).

import XCTest

@testable import GenesisDBTypes

final class WireFormatTests: XCTestCase {
    let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.outputFormatting = [.sortedKeys]
        return e
    }()
    let decoder = JSONDecoder()

    func testNodeInputEncodesSnakeCaseKeys() throws {
        let input = NodeInput(
            labels: ["Person"],
            validFrom: "2026-07-03T00:00:00Z",
            causedBy: "node-1"
        )
        let data = try encoder.encode(input)
        let encoded = String(data: data, encoding: .utf8)!

        XCTAssertTrue(encoded.contains("\"valid_from\""), "expected valid_from, got: \(encoded)")
        XCTAssertTrue(encoded.contains("\"caused_by\""), "expected caused_by, got: \(encoded)")
        XCTAssertFalse(encoded.contains("validFrom"))
        XCTAssertFalse(encoded.contains("causedBy"))
    }

    func testHybridSearchInputEncodesSnakeCaseKeys() throws {
        let input = HybridSearchInput(queryVector: [0.9, 0.1, 0.0], k: 5, efSearch: 64)
        let data = try encoder.encode(input)
        let encoded = String(data: data, encoding: .utf8)!

        XCTAssertTrue(encoded.contains("\"query_vector\""))
        XCTAssertTrue(encoded.contains("\"ef_search\""))
    }

    func testNodeOutputDecodesARealEngineShapedPayload() throws {
        // Shape mirrors what src/ffi.rs's genesisdb_add_node / the REST
        // server actually return (tests/rest_api_tests.rs exercises the same
        // serde structs over HTTP) — snake_case, nested `clock` object.
        let wire = """
            {
              "id": "n1",
              "labels": ["Person"],
              "props": {"name": "Ada"},
              "impact": 0.5,
              "valid_from": "2026-07-03T00:00:00Z",
              "valid_to": null,
              "caused_by": null,
              "expires_at": null,
              "clock": {"time": 3, "peer_id": "peer-a"},
              "collection": "default"
            }
            """
        let node = try decoder.decode(NodeOutput.self, from: Data(wire.utf8))

        XCTAssertEqual(node.id, "n1")
        XCTAssertEqual(node.validFrom, "2026-07-03T00:00:00Z")
        XCTAssertEqual(node.clock.time, 3)
        XCTAssertEqual(node.clock.peerId, "peer-a")
        XCTAssertEqual(node.props["name"]?.stringValue, "Ada")
    }

    func testContextPackageDecodesNestedNodesEdgesSuperNodesAndCoverage() throws {
        let wire = """
            {
              "nodes": [],
              "edges": [],
              "super_nodes": [
                {"cluster_id": 1, "theme": "t", "member_count": 2, "impact": 0.1,
                 "centroid": [0.1, 0.2], "timestamp": "2026-07-03T00:00:00Z"}
              ],
              "token_estimate": 42,
              "reasoning_path": "H1",
              "coverage": {
                "hops_requested": 1, "hops_served": 1,
                "ceiling_hit": false, "truncated": false
              }
            }
            """
        let pkg = try decoder.decode(ContextPackage.self, from: Data(wire.utf8))

        XCTAssertEqual(pkg.tokenEstimate, 42)
        XCTAssertEqual(pkg.reasoningPath, "H1")
        XCTAssertEqual(pkg.superNodes.count, 1)
        XCTAssertEqual(pkg.superNodes[0].clusterId, 1)
        XCTAssertEqual(pkg.superNodes[0].memberCount, 2)
        XCTAssertEqual(pkg.coverage.hopsRequested, 1)
        XCTAssertFalse(pkg.coverage.ceilingHit)
    }

    func testRetrieveContextInputOmitsDefaultedFieldsMatchingEngineSerdeDefaults() throws {
        // Mirrors the Rust `RetrieveContextInput` in src/ffi.rs/src/jni.rs:
        // `tier` is `#[serde(default = "default_tier")]` (-> "H1"), and
        // `budget`/`fuzzy` default to absent/false — so an absent key on the
        // wire resolves to the exact same default on the Rust side. Only the
        // non-defaulted `target_id` is guaranteed to appear.
        let encoded = try String(
            data: encoder.encode(RetrieveContextInput(targetId: "n1", tier: "H1", budget: nil, fuzzy: false)),
            encoding: .utf8
        )!
        XCTAssertTrue(encoded.contains("\"target_id\":\"n1\""))
        XCTAssertFalse(encoded.contains("tier"), "defaulted fields should be omitted, got: \(encoded)")

        // A non-default tier IS sent explicitly.
        let encodedH2 = try String(
            data: encoder.encode(RetrieveContextInput(targetId: "n1", tier: "H2", budget: nil, fuzzy: false)),
            encoding: .utf8
        )!
        XCTAssertTrue(encodedH2.contains("\"tier\":\"H2\""))
    }
}
