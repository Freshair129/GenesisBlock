package main

import (
	"context"
	"fmt"
	"log"

	"github.com/freshair129/genesisblock-go"
)

func main() {
	// Initialize client (assumes GenesisDB is running at localhost:3000)
	client := genesisdb.NewClient("http://localhost:3000")
	ctx := context.Background()

	// 1. Add knowledge
	fmt.Println("Adding node...")
	node, err := client.AddNode(ctx, genesisdb.NodeInput{
		Labels: []string{"GO_TEST"},
		Props: map[string]interface{}{
			"message": "Hello from Go SDK",
		},
	})
	if err != nil {
		log.Fatalf("failed to add node: %v", err)
	}
	fmt.Printf("Node created with ID: %s\n", node.ID)

	// 2. Retrieve tiered context
	fmt.Println("\nRetrieving H1 context...")
	pkg, err := client.GetContext(ctx, node.ID, "H1", nil)
	if err != nil {
		log.Fatalf("failed to get context: %v", err)
	}
	fmt.Printf("Found %d nodes in context. Reasoning Path: %s\n", len(pkg.Nodes), pkg.ReasoningPath)

	// 3. Execute raw HQL
	fmt.Println("\nRunning raw HQL query...")
	res, err := client.Query(ctx, "SEARCH Node SIMILAR TO [0.1, 0.2] K 1")
	if err != nil {
		log.Fatalf("query failed: %v", err)
	}
	fmt.Printf("Query results: %v\n", res)
}
