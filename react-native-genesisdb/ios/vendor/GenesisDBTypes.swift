// GENERATED FILE — DO NOT EDIT.
//
// Copied from ios/genesisdb/Sources/GenesisDBTypes/Types.swift by scripts/vendor-rn-ios-sdk.mjs.
// Edit the original there; CI job `rn-ios-vendor-freshness` fails if this
// copy drifts. See that script's header for why the copy exists at all.

// Wire-format data types for the GenesisBlockDB iOS SDK (MARK XVI Phase B-1).
//
// IMPORTANT: the C ABI (src/ffi.rs) serializes the *same* Rust structs
// (`NodeInput`, `NodeOutput`, ...) as the REST server (src/router.rs) and the
// Android JNI bridge (src/jni.rs), via plain `serde_json` with NO
// `rename_all` attribute. That means the wire format is the literal Rust
// field names, i.e. **snake_case** (`valid_from`, `query_vector`,
// `super_nodes`, ...), NOT the camelCase seen in index.d.ts — that camelCase
// is a napi-rs binding-generation convention specific to the Node addon and
// does not apply here. Every type below carries an explicit `CodingKeys` for
// the true wire key so the public Swift API can stay idiomatic camelCase
// without silently breaking on the engine's actual JSON shape. This mirrors
// android/genesisdb/.../Types.kt's `@SerialName` convention exactly — keep
// the two in lockstep when the Rust structs change.
//
// This target has ZERO dependency on the C ABI / xcframework: it is pure
// Swift + Foundation Codable, so `swift test` can exercise the wire contract
// on any macOS runner without a compiled `.a`/xcframework in the loop — the
// same "host-verifiable, no native lib" property android-jvm-tests has for
// Types.kt / WireFormatTest.kt.

import Foundation

public struct NodeInput: Codable, Sendable {
    public var id: String?
    public var labels: [String]
    public var props: JSONValue?
    public var embedding: [Double]?
    public var lang: String?
    public var validFrom: String?
    public var causedBy: String?
    public var ttl: Int?
    public var collection: String?

    public init(
        id: String? = nil,
        labels: [String],
        props: JSONValue? = nil,
        embedding: [Double]? = nil,
        lang: String? = nil,
        validFrom: String? = nil,
        causedBy: String? = nil,
        ttl: Int? = nil,
        collection: String? = nil
    ) {
        self.id = id
        self.labels = labels
        self.props = props
        self.embedding = embedding
        self.lang = lang
        self.validFrom = validFrom
        self.causedBy = causedBy
        self.ttl = ttl
        self.collection = collection
    }

    enum CodingKeys: String, CodingKey {
        case id, labels, props, embedding, lang
        case validFrom = "valid_from"
        case causedBy = "caused_by"
        case ttl, collection
    }
}

public struct LogicalClock: Codable, Sendable {
    public var time: Int
    public var peerId: String

    public init(time: Int, peerId: String) {
        self.time = time
        self.peerId = peerId
    }

    enum CodingKeys: String, CodingKey {
        case time
        case peerId = "peer_id"
    }
}

public struct NodeOutput: Codable, Sendable {
    public var id: String
    public var labels: [String]
    public var props: JSONValue
    public var impact: Double?
    public var embedding: [Double]?
    public var lang: String?
    public var validFrom: String
    public var validTo: String?
    public var causedBy: String?
    public var expiresAt: String?
    public var clock: LogicalClock
    public var collection: String?

    // NOTE on every explicit `public init` below (NodeOutput, EdgeOutput,
    // SuperNode, CoverageReport, ContextPackage, NeighborOutput): Swift's
    // auto-synthesized memberwise initializer for a struct is always
    // `internal`, even when the struct and every stored property are
    // `public` — unlike Kotlin, where a `data class`'s primary constructor
    // visibility matches the class's own. Without these, a consumer outside
    // this module could decode these types from JSON but never construct one
    // by hand (e.g. for a preview or a hand-written test double).
    public init(
        id: String,
        labels: [String],
        props: JSONValue,
        impact: Double? = nil,
        embedding: [Double]? = nil,
        lang: String? = nil,
        validFrom: String,
        validTo: String? = nil,
        causedBy: String? = nil,
        expiresAt: String? = nil,
        clock: LogicalClock,
        collection: String? = nil
    ) {
        self.id = id
        self.labels = labels
        self.props = props
        self.impact = impact
        self.embedding = embedding
        self.lang = lang
        self.validFrom = validFrom
        self.validTo = validTo
        self.causedBy = causedBy
        self.expiresAt = expiresAt
        self.clock = clock
        self.collection = collection
    }

    enum CodingKeys: String, CodingKey {
        case id, labels, props, impact, embedding, lang
        case validFrom = "valid_from"
        case validTo = "valid_to"
        case causedBy = "caused_by"
        case expiresAt = "expires_at"
        case clock, collection
    }
}

public struct EdgeOutput: Codable, Sendable {
    public var id: String
    public var from: String
    public var to: String
    public var rel: String
    public var props: JSONValue
    public var validFrom: String
    public var validTo: String?
    public var recordedAt: String
    public var supersededBy: String?
    public var impact: Double?
    public var causedBy: String?
    public var clock: LogicalClock

    public init(
        id: String,
        from: String,
        to: String,
        rel: String,
        props: JSONValue,
        validFrom: String,
        validTo: String? = nil,
        recordedAt: String,
        supersededBy: String? = nil,
        impact: Double? = nil,
        causedBy: String? = nil,
        clock: LogicalClock
    ) {
        self.id = id
        self.from = from
        self.to = to
        self.rel = rel
        self.props = props
        self.validFrom = validFrom
        self.validTo = validTo
        self.recordedAt = recordedAt
        self.supersededBy = supersededBy
        self.impact = impact
        self.causedBy = causedBy
        self.clock = clock
    }

    enum CodingKeys: String, CodingKey {
        case id, from, to, rel, props
        case validFrom = "valid_from"
        case validTo = "valid_to"
        case recordedAt = "recorded_at"
        case supersededBy = "superseded_by"
        case impact
        case causedBy = "caused_by"
        case clock
    }
}

public struct SuperNode: Codable, Sendable {
    public var clusterId: Int
    public var theme: String
    public var memberCount: Int
    public var impact: Double
    public var centroid: [Double]
    public var timestamp: String
    public var drift: Double?

    public init(
        clusterId: Int,
        theme: String,
        memberCount: Int,
        impact: Double,
        centroid: [Double],
        timestamp: String,
        drift: Double? = nil
    ) {
        self.clusterId = clusterId
        self.theme = theme
        self.memberCount = memberCount
        self.impact = impact
        self.centroid = centroid
        self.timestamp = timestamp
        self.drift = drift
    }

    enum CodingKeys: String, CodingKey {
        case clusterId = "cluster_id"
        case theme
        case memberCount = "member_count"
        case impact, centroid, timestamp, drift
    }
}

/// Factual retrieval-coverage report for a `ContextPackage`: how much of the
/// requested radius the engine actually served, whether it stopped at the
/// tier boundary with graph still beyond it, and whether budget compression
/// replaced atoms. Mirrors `CoverageReport` in src/lib.rs exactly — every
/// field here is a measurement, not policy (see the Rust doc comment).
public struct CoverageReport: Codable, Sendable {
    public var hopsRequested: Int
    public var hopsServed: Int
    public var ceilingHit: Bool
    public var truncated: Bool

    public init(hopsRequested: Int, hopsServed: Int, ceilingHit: Bool, truncated: Bool) {
        self.hopsRequested = hopsRequested
        self.hopsServed = hopsServed
        self.ceilingHit = ceilingHit
        self.truncated = truncated
    }

    enum CodingKeys: String, CodingKey {
        case hopsRequested = "hops_requested"
        case hopsServed = "hops_served"
        case ceilingHit = "ceiling_hit"
        case truncated
    }
}

public struct ContextPackage: Codable, Sendable {
    public var nodes: [NodeOutput]
    public var edges: [EdgeOutput]
    public var superNodes: [SuperNode]
    public var tokenEstimate: Int
    public var reasoningPath: String
    /// Factual retrieval-coverage signal. See `CoverageReport`. NOTE: this
    /// field is present on the Rust struct (src/lib.rs `ContextPackage`) but
    /// was missing from the Android SDK's Types.kt when B-1 was written —
    /// fixed here rather than copied forward, since `ignoreUnknownKeys`-style
    /// leniency would otherwise silently drop coverage data for every caller.
    public var coverage: CoverageReport

    public init(
        nodes: [NodeOutput],
        edges: [EdgeOutput],
        superNodes: [SuperNode],
        tokenEstimate: Int,
        reasoningPath: String,
        coverage: CoverageReport
    ) {
        self.nodes = nodes
        self.edges = edges
        self.superNodes = superNodes
        self.tokenEstimate = tokenEstimate
        self.reasoningPath = reasoningPath
        self.coverage = coverage
    }

    enum CodingKeys: String, CodingKey {
        case nodes, edges
        case superNodes = "super_nodes"
        case tokenEstimate = "token_estimate"
        case reasoningPath = "reasoning_path"
        case coverage
    }
}

public struct NeighborOutput: Codable, Sendable {
    public var node: NodeOutput
    public var path: [EdgeOutput]
    public var depth: Int
    public var score: Double?

    public init(node: NodeOutput, path: [EdgeOutput], depth: Int, score: Double? = nil) {
        self.node = node
        self.path = path
        self.depth = depth
        self.score = score
    }
}

public struct HybridSearchInput: Codable, Sendable {
    public var queryVector: [Double]
    public var k: Int
    public var alpha: Double?
    public var lang: String?
    public var asOf: String?
    public var collection: String?
    public var efSearch: Int?
    public var oversample: Int?

    public init(
        queryVector: [Double],
        k: Int,
        alpha: Double? = nil,
        lang: String? = nil,
        asOf: String? = nil,
        collection: String? = nil,
        efSearch: Int? = nil,
        oversample: Int? = nil
    ) {
        self.queryVector = queryVector
        self.k = k
        self.alpha = alpha
        self.lang = lang
        self.asOf = asOf
        self.collection = collection
        self.efSearch = efSearch
        self.oversample = oversample
    }

    enum CodingKeys: String, CodingKey {
        case queryVector = "query_vector"
        case k, alpha, lang
        case asOf = "as_of"
        case collection
        case efSearch = "ef_search"
        case oversample
    }
}

/// Payload for `genesisdb_retrieve_context` — mirrors the
/// `RetrieveContextInput` struct defined identically in src/ffi.rs and
/// src/jni.rs (there is no public Rust type of this name; every mobile
/// surface defines it locally). `public`, not internal-to-this-module-only:
/// Swift's `internal` is scoped to the whole MODULE, not the file, and
/// `GenesisDB.swift` constructs this from a *different* SPM target
/// (`GenesisDB` depends on `GenesisDBTypes`, but a dependency does not see a
/// dependency's internal members) — same reasoning as the explicit `public
/// init`s above, since the auto-synthesized memberwise init is internal-only
/// regardless of the type's own access level.
public struct RetrieveContextInput: Encodable, Sendable {
    public var targetId: String
    public var tier: String
    public var budget: Int?
    public var fuzzy: Bool

    public init(targetId: String, tier: String = "H1", budget: Int? = nil, fuzzy: Bool = false) {
        self.targetId = targetId
        self.tier = tier
        self.budget = budget
        self.fuzzy = fuzzy
    }

    enum CodingKeys: String, CodingKey {
        case targetId = "target_id"
        case tier, budget, fuzzy
    }

    /// Matches the Rust side's `#[serde(default)]`/`#[serde(default =
    /// "default_tier")]` field defaults: `tier` omits when it's the default
    /// "H1", `budget`/`fuzzy` omit at their zero values — so a caller who
    /// only sets `targetId` produces a wire payload the engine resolves to
    /// the identical default, matching `encodeDefaults = false` on the
    /// Kotlin side (see WireFormatTest.kt).
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(targetId, forKey: .targetId)
        if tier != "H1" {
            try container.encode(tier, forKey: .tier)
        }
        if let budget {
            try container.encode(budget, forKey: .budget)
        }
        if fuzzy {
            try container.encode(fuzzy, forKey: .fuzzy)
        }
    }
}

/// Thrown when a native call fails (engine error or a caught Rust panic — the
/// C ABI returns null/nonzero for both, so the two are indistinguishable from
/// Swift, exactly as on the JNI/Android side).
public struct GenesisDBError: Error, CustomStringConvertible, Sendable {
    public let message: String
    public init(_ message: String) { self.message = message }
    public var description: String { message }
}

/// A minimal untyped JSON value for `props` fields, which the engine treats
/// as an opaque application payload (arbitrary JSON, not a fixed Rust type).
/// Foundation's `Codable` has no built-in "any JSON" type, so this mirrors
/// what `kotlinx.serialization.json.JsonElement` gives the Kotlin side.
public enum JSONValue: Codable, Sendable, Equatable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let v = try? container.decode(Bool.self) {
            self = .bool(v)
        } else if let v = try? container.decode(Double.self) {
            self = .number(v)
        } else if let v = try? container.decode(String.self) {
            self = .string(v)
        } else if let v = try? container.decode([JSONValue].self) {
            self = .array(v)
        } else if let v = try? container.decode([String: JSONValue].self) {
            self = .object(v)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container, debugDescription: "Unsupported JSON value")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null: try container.encodeNil()
        case .bool(let v): try container.encode(v)
        case .number(let v): try container.encode(v)
        case .string(let v): try container.encode(v)
        case .array(let v): try container.encode(v)
        case .object(let v): try container.encode(v)
        }
    }

    /// Convenience accessor mirroring `JsonElement.jsonObject[...].jsonPrimitive.content`
    /// call sites in the Kotlin wire-format tests.
    public subscript(key: String) -> JSONValue? {
        if case .object(let dict) = self { return dict[key] }
        return nil
    }

    public var stringValue: String? {
        if case .string(let s) = self { return s }
        return nil
    }
}
