package dev.genesisblock

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class MavenCentralConsumerTest {
    @Test
    fun publishedArtifactOpensAndWrites() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dbPath = File(context.filesDir, "central-consumer").apply {
            deleteRecursively()
            mkdirs()
        }.absolutePath

        val db = GenesisDB.open(dbPath)
        try {
            val node = db.addNode(NodeInput(id = "central-consumer", labels = listOf("Smoke")))
            assertEquals("central-consumer", node.id)
            assertTrue(node.labels.contains("Smoke"))
        } finally {
            db.close()
        }
    }

    @Test
    fun publishedArtifactPersistsAcrossHandles() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dbPath = File(context.filesDir, "central-persistence").apply {
            deleteRecursively()
            mkdirs()
        }.absolutePath

        val first = GenesisDB.open(dbPath)
        try {
            first.addNode(NodeInput(id = "central-persist", labels = listOf("Smoke")))
        } finally {
            first.close()
        }

        val second = GenesisDB.open(dbPath)
        try {
            val result = second.retrieveContext("central-persist")
            assertTrue(result.nodes.any { it.id == "central-persist" })
        } finally {
            second.close()
        }
    }
}
