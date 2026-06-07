# Software Requirements Document (SRD): Go Client Binding (Mark XI, Step 3)

## 1. Introduction
To support cloud-native infrastructures and high-performance backend systems, we must provide an official **Go Client (SDK)**. This library will enable developers to build robust, concurrent AI applications that leverage GenesisDB for low-latency reasoning and distributed synchronization.

## 2. Functional Requirements

### FR1: Idiomatic Go Interface
- Use standard Go idioms (structs, methods with error returns).
- Provide a `Client` struct that manages the connection to a GenesisDB server.

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
├── models.go      # GKS Schema types (Node, Edge, ContextPackage)
├── hql_types.go   # Type definitions for HQL results
├── go.mod
└── tests/
```

## 2. API Design (Example Usage)
```go
import "github.com/freshair129/genesisdb-go"

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
3.  **Error Handling:** Define specific error types for connection failures and query syntax errors.

---

## 4. Definition of Done (DoD)
1.  [ ] Go library structure and `go.mod` initialized.
2.  [ ] Core methods (`AddNode`, `Query`, `GetContext`) implemented and tested.
3.  [ ] **Integration Test:** A Go test suite successfully interacts with a live GenesisDB server.
4.  [ ] Documentation updated in `docs/GO-SDK-GUIDE.md`.

---
**Please review and approve this Go Binding Specification. I will begin implementation once approved.**
