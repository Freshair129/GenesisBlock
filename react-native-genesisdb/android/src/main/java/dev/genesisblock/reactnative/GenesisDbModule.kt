package dev.genesisblock.reactnative

import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import dev.genesisblock.ContextPackage
import dev.genesisblock.GenesisDB
import dev.genesisblock.HybridSearchInput
import dev.genesisblock.NeighborOutput
import dev.genesisblock.NodeInput
import dev.genesisblock.NodeOutput
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * RN bridge for `dev.genesisblock:genesisdb-android` (Phase B-2). Every
 * method takes/returns the exact snake_case JSON wire shape documented in
 * ../../src/types.ts — this module's only job is: decode into B-2's typed
 * data classes, call the suspend API, encode the result back to a string.
 *
 * Handles never cross the bridge as raw pointers: `open()` returns a small
 * `dbId` int minted here, mapped internally to the real [GenesisDB]
 * instance — see the precision-loss note in ../../src/NativeGenesisDb.ts.
 */
class GenesisDbModule(reactContext: ReactApplicationContext) :
    ReactContextBaseJavaModule(reactContext) {

    @Serializable
    private data class RetrieveContextRequest(
        @SerialName("target_id") val targetId: String,
        val tier: String = "H1",
        val budget: Int? = null,
        val fuzzy: Boolean = false,
    )

    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = false }
    private val instances = ConcurrentHashMap<Int, GenesisDB>()
    private val nextId = AtomicInteger(1)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun getName() = "GenesisDb"

    override fun invalidate() {
        super.invalidate()
        scope.cancel()
        instances.values.forEach { it.close() }
        instances.clear()
    }

    private fun requireInstance(dbId: Int, promise: Promise): GenesisDB? {
        val db = instances[dbId]
        if (db == null) {
            promise.reject("GENESISDB_INVALID_HANDLE", "unknown or closed dbId $dbId")
        }
        return db
    }

    @ReactMethod
    fun open(path: String, promise: Promise) {
        scope.launch {
            try {
                val db = GenesisDB.open(path)
                val id = nextId.getAndIncrement()
                instances[id] = db
                promise.resolve(id)
            } catch (e: Exception) {
                promise.reject("GENESISDB_OPEN_FAILED", e)
            }
        }
    }

    @ReactMethod
    fun close(dbId: Int, promise: Promise) {
        val db = instances.remove(dbId)
        scope.launch {
            // Closing joins the WAL/index threads on the Rust side — run it
            // off Dispatchers.IO like every other method rather than
            // blocking the caller's thread synchronously.
            db?.close()
            promise.resolve(null)
        }
    }

    @ReactMethod
    fun addNode(dbId: Int, jsonInput: String, promise: Promise) {
        val db = requireInstance(dbId, promise) ?: return
        scope.launch {
            try {
                val input = json.decodeFromString(NodeInput.serializer(), jsonInput)
                val output = db.addNode(input)
                promise.resolve(json.encodeToString(NodeOutput.serializer(), output))
            } catch (e: Exception) {
                promise.reject("GENESISDB_ADD_NODE_FAILED", e)
            }
        }
    }

    @ReactMethod
    fun search(dbId: Int, jsonInput: String, promise: Promise) {
        val db = requireInstance(dbId, promise) ?: return
        scope.launch {
            try {
                val input = json.decodeFromString(HybridSearchInput.serializer(), jsonInput)
                val results = db.search(input)
                promise.resolve(json.encodeToString(ListSerializer(NeighborOutput.serializer()), results))
            } catch (e: Exception) {
                promise.reject("GENESISDB_SEARCH_FAILED", e)
            }
        }
    }

    @ReactMethod
    fun executeHql(dbId: Int, query: String, promise: Promise) {
        val db = requireInstance(dbId, promise) ?: return
        scope.launch {
            try {
                val result = db.executeHql(query)
                promise.resolve(result.toString())
            } catch (e: Exception) {
                promise.reject("GENESISDB_EXECUTE_HQL_FAILED", e)
            }
        }
    }

    @ReactMethod
    fun retrieveContext(dbId: Int, jsonInput: String, promise: Promise) {
        val db = requireInstance(dbId, promise) ?: return
        scope.launch {
            try {
                val req = json.decodeFromString(RetrieveContextRequest.serializer(), jsonInput)
                val pkg: ContextPackage = db.retrieveContext(req.targetId, req.tier, req.budget, req.fuzzy)
                promise.resolve(json.encodeToString(ContextPackage.serializer(), pkg))
            } catch (e: Exception) {
                promise.reject("GENESISDB_RETRIEVE_CONTEXT_FAILED", e)
            }
        }
    }

    @ReactMethod
    fun flushIndex(dbId: Int, promise: Promise) {
        val db = requireInstance(dbId, promise) ?: return
        scope.launch {
            try {
                db.flushIndex()
                promise.resolve(null)
            } catch (e: Exception) {
                promise.reject("GENESISDB_FLUSH_INDEX_FAILED", e)
            }
        }
    }
}
