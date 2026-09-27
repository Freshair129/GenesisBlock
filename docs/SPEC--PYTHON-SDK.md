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
1. **Transport:** The implementation uses `requests` for synchronous HTTP communication with a finite timeout.
2. **Typing:** The implementation uses dataclasses for client models and dictionaries for generic request/response payloads.
3.  **Distribution:** Use pyproject.toml as the authoritative package metadata; keep setup.py only as a compatibility shim. PyPI publication is tag-gated and requires Trusted Publishing configuration.

---

## 4. Current Implementation Status (PR #169)

| Area | Status | Evidence and limits |
|---|---|---|
| Client operations | add_node, query, get_context, execute_query_ir, and query_ir_capabilities are implemented. | Unit tests cover add_node and query in test_client.py, plus Query IR auth, capabilities, structured errors, and timeout behavior in test_client_contract.py. get_context has no dedicated test. add_edge and a direct context-retrieval endpoint are not implemented. |
| Connection management | Base URL trailing slashes are normalized; timeout defaults to 10 seconds and must be positive; API key is optional. | Requests send a Bearer token when configured. The constructor connection check is a no-op; health checks and version verification remain open. |
| Error handling | Request transport exceptions raise ConnectionError; non-2xx and invalid JSON responses raise QueryError. | QueryError exposes status_code, code, and message. The no-op constructor check does not establish reachability. |
| Embeddings | add_node accepts a list of float values. | No NumPy dependency or NumPy-specific conversion is provided. |
| Package metadata | Distribution is genesisblockdb-client 0.1.0; imports remain under genesisdb; Python requirement is 3.10 or newer. | genesisdb-python/pyproject.toml is authoritative and setup.py delegates to it. |
| Package validation | The Python Distribution workflow passed on the pre-merge head e524c323. | That run covered wheel and source distribution builds, clean installs, and unit tests on Python 3.10-3.13. This is prior-head evidence; read the merged head's hosted checks at their exact SHA. The live smoke test adds a node and checks its fields but does not retrieve a node. |
| Publishing | A Trusted Publishing workflow accepts python-v* tags and checks the tag against the package version. | `genesisblockdb-client==0.1.0` is published on PyPI. The tag publish passed in [run 36305930749](https://github.com/Freshair129/GenesisBlock/actions/runs/36305930749); a clean public registry consumer passed in [run 36322437144](https://github.com/Freshair129/GenesisBlock/actions/runs/36322437144). |

## 5. Definition of Done (DoD)

1.  [x] Python library structure established under genesisdb-python.
2. [x] Client methods add_node, query, get_context, execute_query_ir, and query_ir_capabilities are implemented. Unit tests cover add_node, query, and Query IR/auth/error/timeout behavior; get_context has no dedicated test.
3.  [ ] Full live integration adds and retrieves a node. PR #169's live smoke test adds a node and validates the response, but does not retrieve it.
4.  [x] Documentation updated in docs/PYTHON-SDK-GUIDE.md and genesisdb-python/README.md.

PR #169 is the packaging and distribution slice; the open live-retrieval item remains outside its passing pre-merge CI evidence.
