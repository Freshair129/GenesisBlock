package dev.genesisblock

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement

/**
 * Embedded GenesisBlockDB, backed by the JNI bridge in `src/jni.rs`
 * (`dev.genesisblock.GenesisDB` symbol namespace — renaming this class or
 * package requires renaming the `Java_dev_genesisblock_GenesisDB_native*`
 * symbols in lockstep). See docs/SPEC--MOBILE-SDK.md §B-2.
 *
 * One instance owns one native handle (an `Arc<Storage>` boxed on the Rust
 * side). Not thread-safe to call concurrently against the same instance from
 * multiple coroutines racing `close()` — callers should treat an instance the
 * way they'd treat a `Closeable` database connection.
 */
class GenesisDB private constructor(private var handle: Long) : AutoCloseable {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    /** Add a node. Returns the persisted [NodeOutput] (server-assigned `id`,
     * clock, etc. when not supplied). */
    suspend fun addNode(input: NodeInput): NodeOutput = withContext(Dispatchers.IO) {
        val h = requireOpen()
        val payload = json.encodeToString(NodeInput.serializer(), input)
        val result = nativeAddNode(h, payload)
            ?: throw GenesisDBException("addNode failed")
        json.decodeFromString(NodeOutput.serializer(), result)
    }

    /** Hybrid (vector + lexical) search. */
    suspend fun search(input: HybridSearchInput): List<NeighborOutput> = withContext(Dispatchers.IO) {
        val h = requireOpen()
        val payload = json.encodeToString(HybridSearchInput.serializer(), input)
        val result = nativeSearch(h, payload)
            ?: throw GenesisDBException("search failed")
        json.decodeFromString(ListSerializer(NeighborOutput.serializer()), result)
    }

    /** Execute a raw HQL query string. The result shape varies by command
     * (SEARCH/TRAVERSE/MATCH/CONTEXT), so it is returned as a [JsonElement]
     * for the caller to destructure. */
    suspend fun executeHql(query: String): JsonElement = withContext(Dispatchers.IO) {
        val h = requireOpen()
        val result = nativeExecuteHql(h, query)
            ?: throw GenesisDBException("executeHql failed")
        json.parseToJsonElement(result)
    }

    /** Retrieve a tiered [ContextPackage] rooted at `targetId`. `tier` is one
     * of H0..H5 (default H1); see the HQL `CONTEXT` command. */
    suspend fun retrieveContext(
        targetId: String,
        tier: String = "H1",
        budget: Int? = null,
        fuzzy: Boolean = false,
    ): ContextPackage = withContext(Dispatchers.IO) {
        val h = requireOpen()
        val payload = json.encodeToString(
            RetrieveContextInput.serializer(),
            RetrieveContextInput(targetId, tier, budget, fuzzy),
        )
        val result = nativeRetrieveContext(h, payload)
            ?: throw GenesisDBException("retrieveContext failed")
        json.decodeFromString(ContextPackage.serializer(), result)
    }

    /** Flush the async HNSW index so recently added vectors become
     * searchable (read-your-write). */
    suspend fun flushIndex() = withContext(Dispatchers.IO) {
        val h = requireOpen()
        val code = nativeFlushIndex(h)
        if (code != 0) throw GenesisDBException("flushIndex failed (code=$code)")
    }

    /** Close the underlying native handle. Safe to call more than once. Any
     * in-flight call on another coroutine when this runs is undefined
     * behaviour on the native side — callers must not race close() against
     * other methods. */
    override fun close() {
        val h = handle
        if (h != 0L) {
            handle = 0L
            nativeClose(h)
        }
    }

    private fun requireOpen(): Long {
        val h = handle
        check(h != 0L) { "GenesisDB handle is closed" }
        return h
    }

    companion object {
        init {
            System.loadLibrary("genesis_block_native")
        }

        /** Open or create a GenesisDB rooted at `path` (typically
         * `context.filesDir.resolve("genesisdb")` — see
         * docs/SPEC--MOBILE-SDK.md §0-D for the sandbox-path convention). */
        suspend fun open(path: String): GenesisDB = withContext(Dispatchers.IO) {
            val h = nativeOpen(path)
            check(h != 0L) { "Failed to open GenesisDB at $path" }
            GenesisDB(h)
        }

        @JvmStatic private external fun nativeOpen(path: String): Long
        @JvmStatic private external fun nativeClose(handle: Long)
        @JvmStatic private external fun nativeAddNode(handle: Long, jsonInput: String): String?
        @JvmStatic private external fun nativeSearch(handle: Long, jsonInput: String): String?
        @JvmStatic private external fun nativeExecuteHql(handle: Long, query: String): String?
        @JvmStatic private external fun nativeRetrieveContext(handle: Long, jsonInput: String): String?
        @JvmStatic private external fun nativeFlushIndex(handle: Long): Int
    }
}
