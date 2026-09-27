---
status: current
---

# GenesisBlock Python SDK Guide

## 1. Installation

Install from the repository source:

~~~bash
python -m pip install ./genesisdb-python
~~~

The distribution name is genesisblockdb-client and the import namespace remains genesisdb. The package requires Python 3.10 or newer and uses requests to call a separately running GenesisBlockDB server.

The repository includes a PyPI Trusted Publishing workflow. A release requires a python-v<version> tag matching pyproject.toml and a configured PyPI Trusted Publisher bound to the pypi GitHub Environment. Do not treat the registry install command as available until a release has been published and verified from a clean environment.

## 2. Getting Started

The Python SDK allows you to interact with a running GenesisBlockDB server.

~~~python
from genesisdb import GenesisClient

# Initialize client
client = GenesisClient(
    "http://localhost:3000",
    timeout=10.0,
    api_key="local-shared-secret",
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
~~~

## 3. Current Client Behavior

- The client removes trailing slashes from the base URL. The timeout defaults to 10 seconds and must be positive. An optional API key is sent as a Bearer token.
- Construction calls a no-op connection check, so it does not verify server reachability. Health checks and version verification are not implemented.
- add_node sends a node request and returns a Node dataclass. Embeddings are accepted as a list of floats.
- query sends HQL to the REST endpoint and returns decoded JSON.
- execute_query_ir posts a Query IR request to /v1/query/ir. query_ir_capabilities gets the capability manifest from /v1/query/ir/capabilities.
- get_context builds a CONTEXT HQL command and maps the result to a ContextPackage.
- add_edge and NumPy-specific embedding conversion are not implemented.

## 4. Data Models

The SDK provides Node, Edge, and ContextPackage dataclasses.

## 5. Transport and Error Handling

Every request uses the configured timeout and sends an `Authorization: Bearer <key>` header when `api_key` is configured.

Network request failures raise ConnectionError. The constructor's no-op connection check does not raise this error. Non-2xx responses raise QueryError with status_code, code, and message fields. Invalid JSON responses also raise QueryError.
