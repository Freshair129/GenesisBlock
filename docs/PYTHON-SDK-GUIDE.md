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

## 3. Current Client Behavior

- GenesisClient normalizes trailing slashes in the server URL. Construction does not check server reachability.
- add_node sends a node request and returns a Node dataclass. Embeddings are accepted as a list of floats.
- query sends HQL to the REST endpoint and returns the decoded JSON response.
- get_context builds a CONTEXT HQL command and maps the result to a ContextPackage.
- add_edge, NumPy-specific embedding conversion, server health checks, and version verification are not implemented.

## 4. Data Models
The SDK provides Node, Edge, and ContextPackage dataclasses.

## 5. Error Handling

Non-200 responses from query and add_node raise QueryError. Network exceptions from requests currently propagate from the client methods. The custom ConnectionError is not raised by the constructor's no-op connection check.
