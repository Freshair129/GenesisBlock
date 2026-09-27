---
status: current
---

# Software Requirements Document (SRD): Go Client Binding (Mark XI, Step 3)

## 1. Introduction
To support cloud-native infrastructures and high-performance backend systems, we must provide an official **Go Client (SDK)**. This library will enable developers to build robust, concurrent AI applications that leverage GenesisBlockDB for low-latency reasoning and distributed synchronization.

## 2. Functional Requirements

### FR1: Idiomatic Go Interface
- Use standard Go idioms (structs, methods with error returns).
- Provide a `Client` struct that manages the connection to a GenesisBlockDB server.

### FR2: Semantic Operations
- Support `AddNode` and `AddEdge` with JSON-to-Struct mapping.
- Implement `ExecuteHQL` to return results as `[]map[string]interface{}` or typed slices.
- Integrate `GetContext` using the H0-H5 Context Scaling Tier protocol.

### FR3: Concurrency Safety
- Ensure the client is safe for use across multiple goroutines.
- Use `context.Context` for all network requests to support cancellation and timeouts.

---

# Technical Design Document (TDD): Go SDK

## 1. Package Structure
```text
genesisdb-go/
├── client.go      # Main Client struct and logic
├── models.go      # Generic GenesisBlockDB types (Node, Edge, ContextPackage)
├── client_test.go # HTTP client unit tests
├── go.mod
├── README.md      # Module path, install, and usage
└── examples/main.go
```

## 2. API Design (Example Usage)
```go
import "github.com/Freshair129/GenesisBlock/genesisdb-go"

client := genesisdb.NewClient("http://localhost:3000")

// Add Node
node, err := client.AddNode(ctx, genesisdb.NodeInput{
    Labels: []string{"PROCESS"},
    Props: map[string]interface{}{"status": "active"},
})

// Tiered Context
pkg, err := client.GetContext(ctx, "target-id", genesisdb.H1, nil)

// Raw HQL
res, err := client.Query(ctx, "SEARCH Node SIMILAR TO [0.1, 0.2] K 1")
```

## 3. Implementation Strategy
1.  **Transport:** Use the standard `net/http` package for zero-dependency core communication.
2.  **Serialization:** Use `encoding/json` for schema mapping.
3.  **Error Handling:** Return request and decoding errors to callers.
4.  **Distribution:** Keep the module in the monorepo at github.com/Freshair129/GenesisBlock/genesisdb-go. Use a Go submodule tag such as genesisdb-go/v0.1.0 only after the distribution slice is merged and the external consumer resolves it.

---

## 4. Current Implementation Status (PR #170)

| Area | Status at PR head ec7e027a | Evidence and limits |
|---|---|---|
| Module identity | Canonical path is github.com/Freshair129/GenesisBlock/genesisdb-go. | genesisdb-go/go.mod matches its monorepo directory. |
| Client operations | Query, AddNode, and GetContext are implemented. | httptest coverage exercises AddNode and Query; GetContext has no dedicated test. AddEdge is not implemented. |
| Request handling | Requests use net/http and accept context.Context. | The client uses a 30-second HTTP timeout and returns request/decoding errors. |
| Distribution validation | Go 1.20 CI tests the module and resolves the PR head from a clean external module; a live-server consumer creates a node. | Both Go SDK Distribution jobs passed at PR head ec7e027a. |
| Release | No semantic-version tag is created by PR #170. | After merge, the documented submodule tag form is genesisdb-go/v0.1.0. Do not describe it as released until external resolution is verified. |

## 5. Definition of Done (DoD)

1.  [x] Go library structure and go.mod initialized at the canonical module path.
2.  [ ] Core methods AddNode, Query, and GetContext implemented and tested. GetContext is implemented but has no dedicated test; AddEdge remains outside the current client.
3.  [x] Live-server integration resolves the module from the PR head and creates a node through an external consumer.
4.  [x] Distribution and usage documentation updated in genesisdb-go/README.md and this specification.

PR #170 implements the monorepo distribution slice. It does not create a release tag or complete the remaining SDK test coverage.
