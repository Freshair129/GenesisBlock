---
status: current
---

# GenesisBlock Python SDK Guide

## 1. Installation
Install from source:
```bash
cd genesisdb-python
pip install .
```

## 2. Getting Started
The Python SDK allows you to interact with a running GenesisBlockDB server.

```python
from genesisdb import GenesisClient

# Initialize client
client = GenesisClient(
    "http://localhost:3000",
    timeout=10.0,
    api_key="local-shared-secret",  # omit when GENESIS_API_KEY is unset
)

# Add knowledge
node = client.add_node(
    labels=["AGENT"],
    props={"role": "reasoner"}
)

# Semantic retrieval with H0-H5 tiers
context = client.get_context(target="agent-1", tier="H1")

# Raw HQL
results = client.query("CONTEXT FOR 'agent-1' TIER H3")

# Typed Query IR context
packet = client.execute_query_ir({
    "contract_version": "query-ir.v1",
    "request_id": "example-context-1",
    "operation": {
        "kind": "context",
        "target_id": "agent-1",
        "tier": "H1",
        "budget": 512,
    },
})

# Discover implemented and fail-closed capabilities
capabilities = client.query_ir_capabilities()
```

## 3. Data Models
The SDK provides `Node`, `Edge`, and `ContextPackage` dataclasses for type safety.

## 4. Transport and Error Handling

The client applies a finite timeout to every request and sends
`Authorization: Bearer <key>` when `api_key` is configured. `QueryError` exposes
`status_code`, `code`, and `message` for structured server failures.

- `ConnectionError`: Server is unreachable.
- `QueryError`: HQL or REST error from the engine.
