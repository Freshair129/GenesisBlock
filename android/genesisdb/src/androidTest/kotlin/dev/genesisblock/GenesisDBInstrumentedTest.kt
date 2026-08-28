package dev.genesisblock

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/**
 * On-device acceptance for the Android SDK.
 *
 * The gap this closes: `WireFormatTest` says of itself that it loads no native
 * library and exercises no JNI call, and it is the only Kotlin test that
 * existed. So `System.loadLibrary("genesis_block_native")`, the JNI symbol
 * binding, and every `native*` entry point had NEVER run on an Android runtime
 * in CI — only the JSON shapes around them had been checked. iOS has had
 * `ios-acceptance-test` since PR #133, which found three real bugs on its first
 * outing; Android had no equivalent.
 *
 * These tests deliberately use no embeddings. A vector would couple the test to
 * the default collection's dimensionality, which is not what is under test
 * here: what is under test is that the packaged native library loads on a real
 * Android runtime and that data survives a close/reopen through the WAL on
 * Android's filesystem.
 */
@RunWith(AndroidJUnit4::class)
class GenesisDBInstrumentedTest {

    private lateinit var dbPath: String

    @Before
    fun setUp() {
        val ctx = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = File(ctx.filesDir, "genesisdb-acceptance")
        dir.deleteRecursively()
        dir.mkdirs()
        dbPath = dir.absolutePath
    }

    /**
     * The one thing nothing else covers: that the .so for THIS device's ABI is
     * in the artifact, loads, and that the JNI symbols resolve. An .aar missing
     * the running ABI fails here with UnsatisfiedLinkError — which is exactly
     * how a consumer on an x86_64 emulator experienced the two-ARM-slice .aar
     * that shipped at v0.2.0.
     */
    @Test
    fun opensAndClosesOnDevice() = runBlocking {
        val db = GenesisDB.open(dbPath)
        try {
            val out = db.addNode(
                NodeInput(id = "acc:open", labels = listOf("Acceptance")),
            )
            assertEquals("acc:open", out.id)
            assertTrue(
                "labels should round-trip through JNI, got ${out.labels}",
                out.labels.contains("Acceptance"),
            )
        } finally {
            db.close()
        }
    }

    /**
     * Durability across handles, on Android's own filesystem. A write that is
     * acknowledged and then lost on reopen is the defect class Slice 0 (#107)
     * was about; nothing had ever checked it on this platform, where the
     * storage semantics are not the desktop's.
     */
    @Test
    fun nodeSurvivesCloseAndReopen() = runBlocking {
        val first = GenesisDB.open(dbPath)
        try {
            first.addNode(
                NodeInput(
                    id = "acc:persist",
                    labels = listOf("Acceptance"),
                    props = buildJsonObject { put("note", JsonPrimitive("on-device")) },
                ),
            )
        } finally {
            first.close()
        }

        val second = GenesisDB.open(dbPath)
        try {
            val ctx = second.retrieveContext("acc:persist")
            assertTrue(
                "node written before close was not found after reopen; got ids " +
                    ctx.nodes.map { it.id },
                ctx.nodes.any { it.id == "acc:persist" },
            )
        } finally {
            second.close()
        }
    }
}
