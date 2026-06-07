const { Server } = require("@modelcontextprotocol/sdk/server/index.js");
const { StdioServerTransport } = require("@modelcontextprotocol/sdk/server/stdio.js");
const {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} = require("@modelcontextprotocol/sdk/types.js");
const { z } = require("zod");
const { GenesisDatabase } = require("../index.js");
const path = require("path");

// 1. Initialize Database
const dbPath = process.env.GENESIS_DB_PATH || path.join(process.cwd(), ".brain/mcp_db");
console.error(`GRL: Initializing GenesisDB at ${dbPath}`);

const db = GenesisDatabase.open({
  path: dbPath,
  pageCacheMb: 128,
  readOnly: false,
  vectorDim: 1536,
});

// 2. Create MCP Server
const server = new Server(
  {
    name: "genesis-block-server",
    version: "2.0.0",
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// 3. Define Tools
const TOOLS = [
  {
    name: "query_hql",
    description: "Executes a raw HQL (Hybrid Query Language) command on the knowledge graph.",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "The HQL command (e.g., SEARCH Node SIMILAR TO [...] K 5)" },
      },
      required: ["query"],
    },
  },
  {
    name: "retrieve_tiered_context",
    description: "Retrieves a knowledge fragment based on the H0-H5 Context Scaling Tier protocol.",
    inputSchema: {
      type: "object",
      properties: {
        target: { type: "string", description: "Node ID or search term." },
        tier: { type: "string", enum: ["H0", "H1", "H2", "H3", "H4", "H5"], description: "The reasoning tier (radius)." },
        budget: { type: "number", description: "Token budget for compression/SuperNode fallback." },
        fuzzy: { type: "boolean", description: "Enable Thai-aware fuzzy matching." },
      },
      required: ["target", "tier"],
    },
  },
  {
    name: "add_knowledge",
    description: "Adds a new node to the knowledge graph with optional metadata and TTL.",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string" },
        labels: { type: "array", items: { type: "string" } },
        props: { type: "object" },
        embedding: { type: "array", items: { type: "number" } },
        ttl: { type: "number", description: "Time-to-live in seconds." },
      },
      required: ["labels"],
    },
  },
];

// 4. Handle Tool Listing
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: TOOLS,
  };
});

// 5. Handle Tool Calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments } = request.params;

  try {
    switch (name) {
      case "query_hql": {
        const result = await db.executeHql(arguments.query);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      }

      case "retrieve_tiered_context": {
        const result = await db.retrieveContext(
          arguments.target,
          arguments.tier,
          arguments.budget || null,
          arguments.fuzzy || false
        );
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      }

      case "add_knowledge": {
        const result = await db.addNode({
          id: arguments.id || null,
          labels: arguments.labels,
          props: arguments.props || null,
          embedding: arguments.embedding || null,
          lang: "en",
          validFrom: null,
          causedBy: "mcp-agent",
          ttl: arguments.ttl || null,
        });
        return { content: [{ type: "text", text: `Knowledge atom added: ${result.id}` }] };
      }

      default:
        throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    return {
      isError: true,
      content: [{ type: "text", text: `Error executing ${name}: ${error.message}` }],
    };
  }
});

// 6. Start Server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("GenesisBlock MCP Server running on Stdio");
}

main().catch((error) => {
  console.error("Fatal error in MCP server:", error);
  process.exit(1);
});
