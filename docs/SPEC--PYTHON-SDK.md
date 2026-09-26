---
status: current
---

# Software Requirements Document (SRD): Python Client Binding (Mark XI, Step 2)

## 1. Introduction
To enable data scientists and AI researchers to use GenesisBlockDB within their native workflows (e.g., Jupyter, LangChain, Autogen), we must provide a high-level **Python Client**. This library will abstract the REST API complexity and provide a Pythonic interface for graph-semantic operations.

## 2. Functional Requirements

### FR1: Connection Management
- Connect to a standalone GenesisBlockDB server via HTTP/REST.
- Support health checks and version verification.

### FR2: Semantic Operations
- Wrapper for `add_node` and `add_edge` with automatic JSON serialization.
- Support for `execute_hql` returning native Python dictionaries/lists.
- Integration of `retrieve_context` using the H0-H5 tier protocol.

### FR3: Vector Support
- Seamless handling of NumPy arrays or lists for embeddings.

---

# Technical Design Document (TDD): Python SDK

## 1. Library Structure
```text
genesisdb-python/
├── genesisdb/
│   ├── __init__.py
│   ├── client.py      # Main GenesisClient class
│   ├── models.py      # Typed models (Pydantic-like)
│   └── exceptions.py
├── tests/
└── examples/
```

## 2. API Design (Example Usage)
```python
from genesisdb import GenesisClient

client = GenesisClient("http://localhost:3000")

# Atomic Knowledge Injection
client.add_node(
    labels=["CONCEPT"],
    props={"name": "Neural Bridge"},
    embedding=[0.1, 0.2, ...]
)

# Tiered Context Retrieval
context = client.get_context(target="Neural Bridge", tier="H1")
print(f"Nodes found: {len(context.nodes)}")

# Raw HQL
results = client.query("TRAVERSE FROM 'Neural Bridge' DEPTH 2 REL ANY")
```

## 3. Implementation Strategy
1.  **Transport:** Use the `httpx` or `requests` library for robust async/sync communication.
2.  **Typing:** Use `TypedDict` or `dataclasses` to provide IDE auto-completion for generic GenesisBlockDB response and mutation types.
3.  **Distribution:** Use pyproject.toml as the authoritative package metadata; keep setup.py only as a compatibility shim. PyPI publication is tag-gated and requires Trusted Publishing configuration.

---

## 4. Current Implementation Status (PR #169)

| Area | Status at PR head e524c323 | Evidence and limits |
|---|---|---|
| Client operations | add_node, query, and get_context are implemented. | Unit tests cover add_node and query; get_context has no dedicated test. add_edge and a direct retrieve-context endpoint are not implemented. |
| Connection management | Base URL trailing slashes are normalized. | The constructor connection check is a no-op; health checks and version verification remain open. |
| Embeddings | add_node accepts a list of float values. | No NumPy dependency or NumPy-specific conversion is provided. |
| Package metadata | Distribution is genesisblockdb-client 0.1.0; imports remain under genesisdb; Python requirement is 3.10 or newer. | genesisdb-python/pyproject.toml is authoritative and setup.py delegates to it. |
| Package validation | Wheel and source distribution build, clean installs, and unit tests run on Python 3.10–3.13. | The live-server workflow creates a node and checks its returned fields; it does not retrieve a node. |
| Publishing | A Trusted Publishing workflow accepts python-v* tags and checks the tag against the package version. | Publication still requires the PyPI Trusted Publisher binding and the pypi GitHub Environment. No first registry release is claimed. |

## 5. Definition of Done (DoD)

1.  [x] Python library structure established under genesisdb-python.
2.  [x] Core methods add_node, query, and get_context implemented. Current unit tests cover add_node and query only.
3.  [ ] Full live integration adds and retrieves a node. PR #169's live smoke test adds a node and validates the response, but does not retrieve it.
4.  [x] Documentation updated in docs/PYTHON-SDK-GUIDE.md and genesisdb-python/README.md.

PR #169 is the packaging and distribution slice; the open items above remain outside its passing CI evidence.
