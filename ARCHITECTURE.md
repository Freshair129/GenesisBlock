# Architecture Entrypoint

This root file is a short architecture index for humans, tools, and agents.

## Read Order

1. Start with the C4 architecture index: [docs/C4--GENESISDB-ARCHITECTURE.md](docs/C4--GENESISDB-ARCHITECTURE.md)
2. Use the parent technical specification for authoritative behavior: [docs/MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)
3. Follow feature-level specs, TDDs, ADRs, and code anchors from the C4 map.

## Current System Shape

GenesisDB is a local-first hybrid knowledge engine optimized for human-machine collaboration. Its main containers are:

- Rust core engine: storage, WAL, HNSW, graph indices, HQL, GRL, governance, CRDT, consensus.
- Axum REST server: `/v1/*` HTTP surface for agents, SDKs, and dashboard.
- N-API package: native Node/TypeScript integration.
- MCP server: LLM tool interface.
- Python and Go SDKs: REST clients.
- Dashboard and Obsidian-facing integrations.

Do not duplicate architecture decisions in this file. Update the C4 map, master spec, ADRs, or feature specs instead.
