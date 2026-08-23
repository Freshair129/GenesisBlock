package dev.genesisblock

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for the Kotlin <-> engine JSON wire contract. These load no
 * native library and exercise no JNI call — they only prove that
 * `Types.kt`'s `@SerialName`s produce/parse the exact snake_case shape the
 * engine's `serde_json` (un-renamed) structs use on the wire (see the
 * "Wire format gotcha" note in android/README.md).
 */
class WireFormatTest {

    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = false }

    @Test
    fun `NodeInput encodes snake_case keys`() {
        val input = NodeInput(
            labels = listOf("Person"),
            validFrom = "2026-07-03T00:00:00Z",
            causedBy = "node-1",
        )
        val encoded = json.encodeToString(NodeInput.serializer(), input)

        assertTrue("expected valid_from, got: $encoded", encoded.contains("\"valid_from\""))
        assertTrue("expected caused_by, got: $encoded", encoded.contains("\"caused_by\""))
        assertTrue(!encoded.contains("validFrom"))
        assertTrue(!encoded.contains("causedBy"))
    }

    @Test
    fun `HybridSearchInput encodes snake_case keys`() {
        val input = HybridSearchInput(queryVector = listOf(0.9, 0.1, 0.0), k = 5, efSearch = 64)
        val encoded = json.encodeToString(HybridSearchInput.serializer(), input)

        assertTrue(encoded.contains("\"query_vector\""))
        assertTrue(encoded.contains("\"ef_search\""))
    }

    @Test
    fun `NodeOutput decodes a real engine-shaped payload`() {
        // Shape mirrors what src/jni.rs's nativeAddNode / the REST server
        // actually return (tests/rest_api_tests.rs exercises the same
        // serde structs over HTTP) — snake_case, nested `clock` object.
        val wire = """
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
        """.trimIndent()

        val node = json.decodeFromString(NodeOutput.serializer(), wire)

        assertEquals("n1", node.id)
        assertEquals("2026-07-03T00:00:00Z", node.validFrom)
        assertEquals(3, node.clock.time)
        assertEquals("peer-a", node.clock.peerId)
        assertEquals("Ada", node.props.jsonObject["name"]?.jsonPrimitive?.content)
    }

    @Test
    fun `ContextPackage decodes nested nodes edges superNodes and coverage`() {
        // `coverage` carries no #[serde(default)] on the Rust struct, so it
        // is always present on the wire — this fixture pins that (see the
        // doc comment on ContextPackage.coverage in Types.kt).
        val wire = """
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
        """.trimIndent()

        val pkg = json.decodeFromString(ContextPackage.serializer(), wire)

        assertEquals(42, pkg.tokenEstimate)
        assertEquals("H1", pkg.reasoningPath)
        assertEquals(1, pkg.superNodes.size)
        assertEquals(1, pkg.superNodes[0].clusterId)
        assertEquals(2, pkg.superNodes[0].memberCount)
        assertEquals(1, pkg.coverage.hopsRequested)
        assertTrue(!pkg.coverage.ceilingHit)
    }

    @Test
    fun `RetrieveContextInput omits defaulted fields, matching the engine's serde defaults`() {
        // `json` (like GenesisDB.kt's own Json instance) sets `encodeDefaults =
        // false`. That's intentional, not lossy: the Rust `RetrieveContextInput`
        // in src/ffi.rs and src/jni.rs marks `tier` with
        // `#[serde(default = "default_tier")]` (-> "H1"), and `budget`/`fuzzy`
        // are `Option<u32>`/`#[serde(default)] bool`, so an absent key on the
        // wire resolves to the exact same default on the Rust side. Only the
        // non-defaulted `target_id` is guaranteed to appear.
        val encoded = json.encodeToString(
            RetrieveContextInput.serializer(),
            RetrieveContextInput(targetId = "n1"),
        )
        assertTrue(encoded.contains("\"target_id\":\"n1\""))
        assertTrue("defaulted fields should be omitted, got: $encoded", !encoded.contains("tier"))

        // A non-default tier IS sent explicitly.
        val encodedH2 = json.encodeToString(
            RetrieveContextInput.serializer(),
            RetrieveContextInput(targetId = "n1", tier = "H2"),
        )
        assertTrue(encodedH2.contains("\"tier\":\"H2\""))
    }
}
