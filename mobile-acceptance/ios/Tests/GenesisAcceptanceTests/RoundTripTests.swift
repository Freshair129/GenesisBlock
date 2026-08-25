import XCTest
import GenesisBlockDB

/// MARK XVI on-device acceptance (issue #125 follow-up): proves the
/// *published* `GenesisBlockDB.xcframework` release asset — not a local
/// monorepo build — is consumable by a genuinely blank SPM package via
/// `.binaryTarget(url:, checksum:)`, and that the linked C ABI (`src/ffi.rs`)
/// actually executes inside the iOS Simulator, not just compiles.
///
/// This calls the raw `genesisdb_*` C functions directly rather than going
/// through `ios/genesisdb`'s `GenesisDB` Swift actor wrapper — a real
/// external consumer pointed only at the published xcframework (with no
/// SPM-registry entry for `ios/genesisdb` itself; see issue #125's
/// deliberately-deferred SPM-registry item) would have to do the same thing
/// until that gap is closed.
final class RoundTripTests: XCTestCase {
    func testOpenAddNodeRetrieveContextClose() throws {
        let dbPath = FileManager.default.temporaryDirectory
            .appendingPathComponent("genesis-acceptance-\(UUID().uuidString)")
            .path

        guard let handle = genesisdb_open(dbPath) else {
            XCTFail("genesisdb_open returned null")
            return
        }
        defer { genesisdb_close(handle) }

        // addNode
        let nodeInput = #"{"labels":["Person"],"props":{"name":"Ada"}}"#
        guard let addNodeOut = genesisdb_add_node(handle, nodeInput) else {
            XCTFail("genesisdb_add_node returned null")
            return
        }
        let addNodeJSON = String(cString: addNodeOut)
        genesisdb_free_string(addNodeOut)

        guard
            let addNodeData = addNodeJSON.data(using: .utf8),
            let addNodeObj = try? JSONSerialization.jsonObject(with: addNodeData) as? [String: Any],
            let nodeId = addNodeObj["id"] as? String
        else {
            XCTFail("could not parse node id from addNode response: \(addNodeJSON)")
            return
        }

        // retrieveContext against the node we just added
        let contextInput = #"{"target_id":"\#(nodeId)","tier":"H1","budget":null,"fuzzy":false}"#
        guard let ctxOut = genesisdb_retrieve_context(handle, contextInput) else {
            XCTFail("genesisdb_retrieve_context returned null")
            return
        }
        let ctxJSON = String(cString: ctxOut)
        genesisdb_free_string(ctxOut)

        XCTAssertTrue(
            ctxJSON.contains(nodeId),
            "retrieveContext response missing the node we just added: \(ctxJSON)"
        )

        // flush_index — read-your-write for the async HNSW indexer.
        XCTAssertEqual(genesisdb_flush_index(handle), 0, "flush_index should succeed")
    }
}
