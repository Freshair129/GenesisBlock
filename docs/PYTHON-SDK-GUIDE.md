# GenesisBlock Python SDK Guide

## 1. Installation
Install from source:
```bash
cd genesisdb-python
pip install .
```

## 2. Getting Started
The Python SDK allows you to interact with a running GenesisDB server.

```python
from genesisdb import GenesisClient

# Initialize client
client = GenesisClient("http://localhost:3000")

# Add knowledge
node = client.add_node(
    labels=["AGENT"],
    props={"role": "reasoner"}
)

# Semantic retrieval with H0-H5 tiers
context = client.get_context(target="agent-1", tier="H1")

# Raw HQL
results = client.query("CONTEXT FOR 'agent-1' TIER H3")
```

## 3. Data Models
The SDK provides `Node`, `Edge`, and `ContextPackage` dataclasses for type safety.

## 4. Error Handling
- `ConnectionError`: Server is unreachable.
- `QueryError`: HQL or REST error from the engine.
