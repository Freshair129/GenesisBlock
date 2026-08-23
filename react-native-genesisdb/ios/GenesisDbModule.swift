import Foundation
import GenesisDB
import GenesisDBTypes

/// RN bridge for `ios/genesisdb`'s `GenesisDB` actor (MARK XVI Phase B-1/B-3).
/// Every method takes/returns the exact snake_case JSON wire shape documented
/// in ../src/types.ts — this module's only job is: decode into GenesisDBTypes,
/// call the actor's `async throws` API, encode the result back to a string.
/// Mirrors `../android/.../GenesisDbModule.kt` method-for-method (open, close,
/// addNode, search, executeHql, retrieveContext, flushIndex).
///
/// Handles never cross the bridge as raw pointers: `open()` returns a small
/// `dbId` int minted by `InstanceRegistry` below, mapped internally to the
/// real `GenesisDB` actor instance — see the precision-loss note in
/// ../src/NativeGenesisDb.ts.
///
/// INTEGRATION NOTE (unlike every other file in this monorepo, this one is
/// NOT verified by any CI job — see react-native-genesisdb/README.md
/// "iOS integration status"): `import GenesisDB` / `import GenesisDBTypes`
/// resolve only if the consuming Xcode project ALSO adds `../ios/genesisdb`
/// as a Swift Package dependency alongside `pod install` — CocoaPods has no
/// mechanism to express a podspec-level dependency on an SPM package, so this
/// is a required manual step for any real integrator today, not a bug here.
@objc(GenesisDb)
class GenesisDbModule: NSObject {

    /// Owns the dbId -> live-instance map. A plain dictionary guarded by a
    /// lock would work too, but an actor is the idiomatic Swift way to get
    /// the same safety Kotlin's `ConcurrentHashMap` + `AtomicInteger` give
    /// `GenesisDbModule.kt` — RN can invoke bridge methods from more than one
    /// queue, so this must not race.
    private actor InstanceRegistry {
        private var instances: [Int: GenesisDB] = [:]
        private var nextId = 1

        func insert(_ db: GenesisDB) -> Int {
            let id = nextId
            nextId += 1
            instances[id] = db
            return id
        }

        /// Returns the instance for `dbId`, or nil if it was never opened or
        /// was already closed/removed.
        func get(_ dbId: Int) -> GenesisDB? {
            instances[dbId]
        }

        /// Removes and returns the instance so the caller can `close()` it
        /// outside this actor (closing doesn't need the registry's lock).
        func remove(_ dbId: Int) -> GenesisDB? {
            instances.removeValue(forKey: dbId)
        }

        func removeAll() -> [GenesisDB] {
            let all = Array(instances.values)
            instances.removeAll()
            return all
        }
    }

    private let registry = InstanceRegistry()
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    @objc static func requiresMainQueueSetup() -> Bool { false }

    /// Every RN app teardown path should close every live handle rather than
    /// leaking native resources — mirrors `GenesisDbModule.kt`'s
    /// `invalidate()` override (this class doesn't subclass `RCTEventEmitter`/
    /// have RN's own `invalidate()` hook, so callers needing this should wire
    /// it to their own app-lifecycle teardown; documented in the README).
    func closeAll() {
        Task {
            let dbs = await registry.removeAll()
            for db in dbs {
                await db.close()
            }
        }
    }

    private func requireInstance(
        _ dbId: NSNumber,
        _ reject: @escaping RCTPromiseRejectBlock
    ) async -> GenesisDB? {
        let id = dbId.intValue
        guard let db = await registry.get(id) else {
            reject("GENESISDB_INVALID_HANDLE", "unknown or closed dbId \(id)", nil)
            return nil
        }
        return db
    }

    @objc
    func open(
        _ path: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            do {
                let db = try GenesisDB.open(path: URL(fileURLWithPath: path))
                let id = await registry.insert(db)
                resolve(id)
            } catch {
                reject("GENESISDB_OPEN_FAILED", "\(error)", error)
            }
        }
    }

    @objc
    func close(
        _ dbId: NSNumber,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            if let db = await registry.remove(dbId.intValue) {
                await db.close()
            }
            resolve(nil)
        }
    }

    @objc
    func addNode(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            guard let db = await requireInstance(dbId, reject) else { return }
            do {
                let input = try decoder.decode(NodeInput.self, from: Data(jsonInput.utf8))
                let output = try await db.addNode(input)
                resolve(String(decoding: try encoder.encode(output), as: UTF8.self))
            } catch {
                reject("GENESISDB_ADD_NODE_FAILED", "\(error)", error)
            }
        }
    }

    @objc
    func search(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            guard let db = await requireInstance(dbId, reject) else { return }
            do {
                let input = try decoder.decode(HybridSearchInput.self, from: Data(jsonInput.utf8))
                let results = try await db.search(input)
                resolve(String(decoding: try encoder.encode(results), as: UTF8.self))
            } catch {
                reject("GENESISDB_SEARCH_FAILED", "\(error)", error)
            }
        }
    }

    @objc
    func executeHql(
        _ dbId: NSNumber,
        query: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            guard let db = await requireInstance(dbId, reject) else { return }
            do {
                let result = try await db.executeHql(query)
                resolve(String(decoding: try encoder.encode(result), as: UTF8.self))
            } catch {
                reject("GENESISDB_EXECUTE_HQL_FAILED", "\(error)", error)
            }
        }
    }

    @objc
    func retrieveContext(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            guard let db = await requireInstance(dbId, reject) else { return }
            do {
                // Wire shape mirrors GenesisDB.kt's RetrieveContextRequest /
                // the engine's RetrieveContextInput exactly: target_id
                // (required), tier/budget/fuzzy (optional, engine-defaulted).
                struct Req: Decodable {
                    let targetId: String
                    let tier: String?
                    let budget: Int?
                    let fuzzy: Bool?
                    enum CodingKeys: String, CodingKey {
                        case targetId = "target_id"
                        case tier, budget, fuzzy
                    }
                }
                let req = try decoder.decode(Req.self, from: Data(jsonInput.utf8))
                let pkg = try await db.retrieveContext(
                    targetId: req.targetId,
                    tier: req.tier ?? "H1",
                    budget: req.budget,
                    fuzzy: req.fuzzy ?? false
                )
                resolve(String(decoding: try encoder.encode(pkg), as: UTF8.self))
            } catch {
                reject("GENESISDB_RETRIEVE_CONTEXT_FAILED", "\(error)", error)
            }
        }
    }

    @objc
    func flushIndex(
        _ dbId: NSNumber,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        Task {
            guard let db = await requireInstance(dbId, reject) else { return }
            do {
                try await db.flushIndex()
                resolve(nil)
            } catch {
                reject("GENESISDB_FLUSH_INDEX_FAILED", "\(error)", error)
            }
        }
    }
}
