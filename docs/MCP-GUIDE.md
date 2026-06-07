# GenesisBlock: Model Context Protocol (MCP) Guide

## 1. Introduction
GenesisBlock implements the **Model Context Protocol (MCP)**, allowing Large Language Models (LLMs) like Claude or ChatGPT to use your local knowledge graph as a high-performance external memory.

## 2. Setup

### 2.1 Prerequisites
- Node.js >= 20
- Rust toolchain (for native bindings)

### 2.2 Installation
```bash
npm install
npm run build
```

### 2.3 Starting the Server
```bash
npm run mcp:start
```
By default, the server initializes a database at `.brain/mcp_db`. You can override this by setting `GENESIS_DB_PATH`.

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
