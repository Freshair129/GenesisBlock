package genesisdb

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestAddNode(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/node/add" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		if r.Method != http.MethodPost {
			t.Fatalf("unexpected method: %s", r.Method)
		}
		var body NodeInput
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		if body.CausedBy != "go-sdk" {
			t.Fatalf("expected default caused_by=go-sdk, got %q", body.CausedBy)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"go-node","labels":["Doc"],"props":{"source":"test"}}`))
	}))
	defer server.Close()

	client := NewClient(server.URL)
	node, err := client.AddNode(context.Background(), NodeInput{Labels: []string{"Doc"}})
	if err != nil {
		t.Fatalf("AddNode: %v", err)
	}
	if node.ID != "go-node" {
		t.Fatalf("unexpected node id: %s", node.ID)
	}
}

func TestQuery(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/query/hql" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"ok":true}`))
	}))
	defer server.Close()

	client := NewClient(server.URL)
	result, err := client.Query(context.Background(), "MATCH (n) RETURN n")
	if err != nil {
		t.Fatalf("Query: %v", err)
	}
	obj, ok := result.(map[string]interface{})
	if !ok || obj["ok"] != true {
		t.Fatalf("unexpected result: %#v", result)
	}
}
