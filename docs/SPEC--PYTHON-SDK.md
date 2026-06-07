# Software Requirements Document (SRD): Python Client Binding (Mark XI, Step 2)

## 1. Introduction
To enable data scientists and AI researchers to use GenesisDB within their native workflows (e.g., Jupyter, LangChain, Autogen), we must provide a high-level **Python Client**. This library will abstract the REST API complexity and provide a Pythonic interface for graph-semantic operations.

## 2. Functional Requirements

### FR1: Connection Management
- Connect to a standalone GenesisDB server via HTTP/REST.
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
2.  **Typing:** Use `TypedDict` or `dataclasses` to provide IDE auto-completion for GKS schemas.
3.  **Deployment:** Prepare a `setup.py` or `pyproject.toml` for future PyPI publishing.

---

## 4. Definition of Done (DoD)
1.  [ ] Python library structure established.
2.  [ ] Core methods (`add_node`, `query`, `get_context`) implemented.
3.  [ ] **Integration Test:** A Python script successfully adds a node and retrieves it from a running GenesisDB server.
4.  [ ] Documentation updated in `docs/PYTHON-SDK-GUIDE.md`.

---
**Please review and approve this Python Binding Specification. I will begin implementation once approved.**
