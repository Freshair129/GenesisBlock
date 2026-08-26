// GENERATED FILE — DO NOT EDIT.
//
// Copied from ios/genesisdb/Sources/GenesisDB/GenesisDB.swift by scripts/vendor-rn-ios-sdk.mjs.
// Edit the original there; CI job `rn-ios-vendor-freshness` fails if this
// copy drifts. See that script's header for why the copy exists at all.

// Embedded GenesisBlockDB, backed by the C ABI in `src/ffi.rs` (MARK XVI
// Phase B-1). See docs/SPEC--MOBILE-SDK.md §B-1.
//
// One instance owns one native handle (an `Arc<Storage>` boxed on the Rust
// side, reached through `genesisdb_open`/`genesisdb_close`). Modeled as a
// Swift `actor` rather than a plain class: every method call is automatically
// serialized onto the actor's executor, so — unlike the Android `GenesisDB`
// class, which documents "not thread-safe to call concurrently... callers
// should treat an instance like a Closeable database connection" as a caller
// obligation — a caller here cannot race `close()` against another method by
// construction. `close()` still makes every subsequent call throw
// `GenesisDBError`, since the native handle is genuinely gone.

import Foundation
import GenesisBlockDB

public actor GenesisDB {
    private var handle: OpaquePointer?

    private static let encoder = JSONEncoder()
    private static let decoder = JSONDecoder()

    private init(handle: OpaquePointer) {
        self.handle = handle
    }

    /// Open or create a GenesisDB rooted at `path` (typically an app-sandbox
    /// directory, e.g. `FileManager.default.urls(for: .applicationSupportDirectory,
    /// in: .userDomainMask)[0].appendingPathComponent("genesisdb")` — see
    /// docs/SPEC--MOBILE-SDK.md §0-D for the sandbox-path convention).
    public static func open(path: URL) throws -> GenesisDB {
        guard let h = path.withUnsafeFileSystemRepresentation({ cPath -> OpaquePointer? in
            guard let cPath else { return nil }
            return genesisdb_open(cPath)
        }) else {
            throw GenesisDBError("Failed to open GenesisDB at \(path.path)")
        }
        return GenesisDB(handle: h)
    }

    /// Close the underlying native handle. Safe to call more than once — a
    /// second call is a no-op (mirrors `genesisdb_close`'s own null-safety).
    public func close() {
        guard let h = handle else { return }
        handle = nil
        genesisdb_close(h)
    }

    deinit {
        // Actors can't run isolated code from `deinit`, so this duplicates
        // `close()`'s null-guard rather than calling it — `genesisdb_close`
        // itself is documented as a safe no-op on an already-freed handle in
        // the ordinary (non-racing) case, but a caller who forgets to call
        // `close()` should still not leak the native handle.
        if let h = handle {
            genesisdb_close(h)
        }
    }

    private func requireOpen() throws -> OpaquePointer {
        guard let h = handle else {
            throw GenesisDBError("GenesisDB handle is closed")
        }
        return h
    }

    /// Add a node. Returns the persisted `NodeOutput` (server-assigned `id`,
    /// clock, etc. when not supplied).
    public func addNode(_ input: NodeInput) throws -> NodeOutput {
        try call(genesisdb_add_node, input: input, as: NodeOutput.self, op: "addNode")
    }

    /// Hybrid (vector + lexical) search.
    public func search(_ input: HybridSearchInput) throws -> [NeighborOutput] {
        try call(genesisdb_search, input: input, as: [NeighborOutput].self, op: "search")
    }

    /// Execute a raw HQL query string. The result shape varies by command
    /// (SEARCH/TRAVERSE/MATCH/CONTEXT), so it is returned as a `JSONValue`
    /// for the caller to destructure.
    public func executeHql(_ query: String) throws -> JSONValue {
        let h = try requireOpen()
        guard let resultPtr = query.withCString({ genesisdb_execute_hql(h, $0) }) else {
            throw GenesisDBError("executeHql failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        return try Self.decoder.decode(JSONValue.self, from: Data(String(cString: resultPtr).utf8))
    }

    /// Retrieve a tiered `ContextPackage` rooted at `targetId`. `tier` is one
    /// of H0..H5 (default H1); see the HQL `CONTEXT` command.
    public func retrieveContext(
        targetId: String,
        tier: String = "H1",
        budget: Int? = nil,
        fuzzy: Bool = false
    ) throws -> ContextPackage {
        let input = RetrieveContextInput(targetId: targetId, tier: tier, budget: budget, fuzzy: fuzzy)
        return try call(
            genesisdb_retrieve_context, input: input, as: ContextPackage.self, op: "retrieveContext")
    }

    /// The Query IR capability manifest (incl. `temporal.history_horizon` and
    /// the retention profile — ADR I6).
    public func queryIrCapabilities() throws -> JSONValue {
        let h = try requireOpen()
        guard let resultPtr = genesisdb_query_ir_capabilities(h) else {
            throw GenesisDBError("queryIrCapabilities failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        return try Self.decoder.decode(JSONValue.self, from: Data(String(cString: resultPtr).utf8))
    }

    /// Execute a versioned Typed Query IR request (contract `query-ir.v1`).
    /// `requestJson` and the returned envelope are both raw JSON — the IR
    /// envelope shape is call-site dependent (search vs traverse), same
    /// reasoning as `executeHql`.
    public func executeQueryIr(_ requestJson: String) throws -> JSONValue {
        try callRaw(genesisdb_execute_query_ir, jsonInput: requestJson, op: "executeQueryIr")
    }

    /// Register a versioned relational schema package encoded as JSON.
    /// Returns the new schema version.
    public func registerRelationalSchema(_ packageJson: String) throws -> Int {
        let h = try requireOpen()
        guard
            let resultPtr = packageJson.withCString({
                genesisdb_register_relational_schema(h, $0)
            })
        else {
            throw GenesisDBError("registerRelationalSchema failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        guard let version = Int(String(cString: resultPtr)) else {
            throw GenesisDBError("registerRelationalSchema returned a non-integer version")
        }
        return version
    }

    /// Return the current relational schema package for a namespace.
    public func getRelationalSchema(namespace: String) throws -> JSONValue {
        let h = try requireOpen()
        guard
            let resultPtr = namespace.withCString({ genesisdb_get_relational_schema(h, $0) })
        else {
            throw GenesisDBError("getRelationalSchema failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        return try Self.decoder.decode(JSONValue.self, from: Data(String(cString: resultPtr).utf8))
    }

    /// Apply an idempotent U2 mutation batch and return its commit receipt.
    public func applyRelationalBatch(_ batchJson: String) throws -> JSONValue {
        try callRaw(genesisdb_apply_relational_batch, jsonInput: batchJson, op: "applyRelationalBatch")
    }

    /// Apply a typed relational mutation group; raw SQL writes are
    /// intentionally unavailable. Unlike every other native call here, the C
    /// ABI returns a plain `Int32` status code (0 = success) rather than a
    /// JSON string — `genesisdb_apply_relational_rows` is the one exception
    /// in src/ffi.rs, so it is NOT routed through `call`/`callRaw`.
    public func applyRelationalRows(_ inputJson: String) throws {
        let h = try requireOpen()
        let code = inputJson.withCString { genesisdb_apply_relational_rows(h, $0) }
        if code != 0 {
            throw GenesisDBError("applyRelationalRows failed (code=\(code))")
        }
    }

    /// Execute a bounded typed relational query and return its JSON row array.
    public func queryRelational(_ queryJson: String) throws -> JSONValue {
        try callRaw(genesisdb_query_relational, jsonInput: queryJson, op: "queryRelational")
    }

    /// Execute a registered named query; arbitrary SQL is never accepted.
    public func executeNamedQuery(_ requestJson: String) throws -> JSONValue {
        try callRaw(genesisdb_execute_named_query, jsonInput: requestJson, op: "executeNamedQuery")
    }

    /// Commit one canonical cross-domain Genesis transaction.
    public func commitTransaction(_ transactionJson: String) throws -> JSONValue {
        try callRaw(genesisdb_commit_transaction, jsonInput: transactionJson, op: "commitTransaction")
    }

    /// Flush the async HNSW index so recently added vectors become
    /// searchable (read-your-write).
    public func flushIndex() throws {
        let h = try requireOpen()
        let code = genesisdb_flush_index(h)
        if code != 0 {
            throw GenesisDBError("flushIndex failed (code=\(code))")
        }
    }

    // MARK: - Shared JSON-in/JSON-out plumbing

    /// Encode `input`, call a `(handle, *const c_char) -> *const c_char` C
    /// function, decode the result as `T`. Every JSON-returning entry point
    /// in src/ffi.rs shares this exact shape (except
    /// `genesisdb_apply_relational_rows`, handled separately above).
    private func call<Input: Encodable, T: Decodable>(
        _ fn: (OpaquePointer?, UnsafePointer<CChar>?) -> UnsafePointer<CChar>?,
        input: Input,
        as _: T.Type,
        op: String
    ) throws -> T {
        let h = try requireOpen()
        let payload = try Self.encoder.encode(input)
        let json = String(decoding: payload, as: UTF8.self)
        guard let resultPtr = json.withCString({ fn(h, $0) }) else {
            throw GenesisDBError("\(op) failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        return try Self.decoder.decode(T.self, from: Data(String(cString: resultPtr).utf8))
    }

    /// Same as `call`, but for the methods whose input is already a raw JSON
    /// string (the relational surface intentionally has no typed Swift input
    /// struct — same choice `GenesisDB.kt` makes, since the caller-facing
    /// contract there is "pass the canonical JSON payload", not a
    /// platform-specific struct).
    private func callRaw(
        _ fn: (OpaquePointer?, UnsafePointer<CChar>?) -> UnsafePointer<CChar>?,
        jsonInput: String,
        op: String
    ) throws -> JSONValue {
        let h = try requireOpen()
        guard let resultPtr = jsonInput.withCString({ fn(h, $0) }) else {
            throw GenesisDBError("\(op) failed")
        }
        defer { genesisdb_free_string(resultPtr) }
        return try Self.decoder.decode(JSONValue.self, from: Data(String(cString: resultPtr).utf8))
    }
}
