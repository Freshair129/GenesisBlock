package dev.genesisblock

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * Wire-format data classes for the GenesisBlockDB Android SDK.
 *
 * IMPORTANT: the JNI bridge (src/jni.rs) and the C ABI (src/ffi.rs) serialize
 * the *same* Rust structs (`NodeInput`, `NodeOutput`, ...) as the REST server
 * (src/router.rs), via plain `serde_json` — with NO `rename_all` attribute.
 * That means the wire format is the literal Rust field names, i.e.
 * **snake_case** (`valid_from`, `query_vector`, `super_nodes`, ...), NOT the
 * camelCase seen in index.d.ts. That camelCase is a napi-rs binding-generation
 * convention specific to the Node addon — it does not apply here. Every field
 * below carries an explicit `@SerialName` for the true wire key so the public
 * Kotlin API can stay idiomatic camelCase without silently breaking on the
 * engine's actual JSON shape. (Verified against tests/rest_api_tests.rs, which
 * exercises the same serde structs over HTTP with snake_case bodies.)
 */

@Serializable
data class NodeInput(
    val id: String? = null,
    val labels: List<String>,
    val props: JsonElement? = null,
    val embedding: List<Double>? = null,
    val lang: String? = null,
    @SerialName("valid_from") val validFrom: String? = null,
    @SerialName("caused_by") val causedBy: String? = null,
    val ttl: Int? = null,
    val collection: String? = null,
)

@Serializable
data class LogicalClock(
    val time: Int,
    @SerialName("peer_id") val peerId: String,
)

@Serializable
data class NodeOutput(
    val id: String,
    val labels: List<String>,
    val props: JsonElement,
    val impact: Double? = null,
    val embedding: List<Double>? = null,
    val lang: String? = null,
    @SerialName("valid_from") val validFrom: String,
    @SerialName("valid_to") val validTo: String? = null,
    @SerialName("caused_by") val causedBy: String? = null,
    @SerialName("expires_at") val expiresAt: String? = null,
    val clock: LogicalClock,
    val collection: String? = null,
)

@Serializable
data class EdgeOutput(
    val id: String,
    val from: String,
    val to: String,
    val rel: String,
    val props: JsonElement,
    @SerialName("valid_from") val validFrom: String,
    @SerialName("valid_to") val validTo: String? = null,
    @SerialName("recorded_at") val recordedAt: String,
    @SerialName("superseded_by") val supersededBy: String? = null,
    val impact: Double? = null,
    @SerialName("caused_by") val causedBy: String? = null,
    val clock: LogicalClock,
)

@Serializable
data class SuperNode(
    @SerialName("cluster_id") val clusterId: Int,
    val theme: String,
    @SerialName("member_count") val memberCount: Int,
    val impact: Double,
    val centroid: List<Double>,
    val timestamp: String,
    val drift: Double? = null,
)

/**
 * Factual retrieval-coverage report for a [ContextPackage]: how much of the
 * requested radius the engine actually served, whether it stopped at the
 * tier boundary with graph still beyond it, and whether budget compression
 * replaced atoms. Mirrors `CoverageReport` in src/lib.rs exactly — every
 * field is a measurement, not policy (see the Rust doc comment).
 */
@Serializable
data class CoverageReport(
    @SerialName("hops_requested") val hopsRequested: Int,
    @SerialName("hops_served") val hopsServed: Int,
    @SerialName("ceiling_hit") val ceilingHit: Boolean,
    val truncated: Boolean,
)

@Serializable
data class ContextPackage(
    val nodes: List<NodeOutput>,
    val edges: List<EdgeOutput>,
    @SerialName("super_nodes") val superNodes: List<SuperNode>,
    @SerialName("token_estimate") val tokenEstimate: Int,
    @SerialName("reasoning_path") val reasoningPath: String,
    /**
     * Factual retrieval-coverage signal. See [CoverageReport]. Was missing
     * entirely until the iOS SDK (B-1) added the Rust struct's `coverage`
     * field to the wire types on both platforms — the Rust field carries no
     * `#[serde(default)]`, so it is always present on the wire; a caller
     * decoding an engine response before this fix silently never saw it
     * (kotlinx.serialization's `ignoreUnknownKeys` swallowed the key rather
     * than failing).
     */
    val coverage: CoverageReport,
)

@Serializable
data class NeighborOutput(
    val node: NodeOutput,
    val path: List<EdgeOutput>,
    val depth: Int,
    val score: Double? = null,
)

@Serializable
data class HybridSearchInput(
    @SerialName("query_vector") val queryVector: List<Double>,
    val k: Int,
    val alpha: Double? = null,
    val lang: String? = null,
    @SerialName("as_of") val asOf: String? = null,
    val collection: String? = null,
    @SerialName("ef_search") val efSearch: Int? = null,
    val oversample: Int? = null,
)

/** Internal-only payload for `nativeRetrieveContext` — mirrors the
 * `RetrieveContextInput` struct defined identically in src/ffi.rs and
 * src/jni.rs (there is no public Rust type of this name; both mobile surfaces
 * define it locally). */
@Serializable
internal data class RetrieveContextInput(
    @SerialName("target_id") val targetId: String,
    val tier: String = "H1",
    val budget: Int? = null,
    val fuzzy: Boolean = false,
)

/** Thrown when a native call fails (engine error or a caught Rust panic —
 * the JNI bridge returns `null`/nonzero for both, so the two are
 * indistinguishable from Kotlin). */
class GenesisDBException(message: String) : Exception(message)
