---
status: current
---

# GenesisBlock: Model Context Protocol (MCP) Guide

## 1. Introduction
GenesisBlock implements the **Model Context Protocol (MCP)**, allowing Large Language Models (LLMs) like Claude or ChatGPT to use your local knowledge graph as a high-performance external memory.

## 2. Setup

### 2.1 Prerequisites
- Node.js >= 20
- Rust toolchain when building the native addon from source; registry installs use prebuilt native packages for supported targets.

### 2.2 Repository installation
```bash
npm install
npm run build
```

### 2.3 Starting the Server
```bash
npm run mcp:start
```
By default, the server initializes a database at `.brain/mcp_db`. You can override this by setting `GENESIS_DB_PATH`.

### 2.4 Registry installation
The `genesisblock-mcp` executable is bundled in the main npm package, `@freshair129/gks-genesis-block-native`; it is not a separate package. The CLI and native engine binding are shipped in the same package release, so a pinned CLI uses the engine version bundled with it. The MCP surface has no independent version.

The published `0.2.5` package predates this executable and does not include it. Until a package version containing `genesisblock-mcp` is published, use the repository-based installation above.

After that version is published, an MCP client can run the registry package without a repository checkout. For example, in Claude Desktop, replace `<version-with-mcp-cli>` with the published package version and set `GENESIS_DB_PATH` to the directory where this server should keep its database:

```json
{
  "mcpServers": {
    "genesisblock": {
      "command": "npx",
      "args": [
        "--yes",
        "--package",
        "@freshair129/gks-genesis-block-native@<version-with-mcp-cli>",
        "genesisblock-mcp"
      ],
      "env": {
        "GENESIS_DB_PATH": "/path/to/agent-memory"
      }
    }
  }
}
```

## 3. Tool Reference

### `query_hql`
Allows the agent to run raw HQL commands.
- **Input:** `query` (string)
- **Output:** JSON results from the engine.

### `retrieve_tiered_context`
The primary tool for RAG. It uses the GRL protocol (H0-H5) to load a relevant sub-graph within a token budget.
- **Input:** 
    - `target`: Node ID or search term.
    - `tier`: "H0" through "H5".
    - `budget`: (Optional) Max token count.
    - `fuzzy`: (Optional) Boolean.

### `add_knowledge`
Allows the agent to save new information into the graph.
- **Input:** 
    - `labels`: Node types.
    - `props`: Metadata JSON.
    - `ttl`: (Optional) Expiration in seconds.

## 4. Integration Example (Claude Desktop)
Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "genesisblock": {
      "command": "node",
      "args": ["G:/GenesisBlock_Dev/GenesisBlock/mcp/server.js"],
      "env": {
        "GENESIS_DB_PATH": "G:/GenesisBlock_Dev/GenesisBlock/.brain/main_db"
      }
    }
  }
}
```
