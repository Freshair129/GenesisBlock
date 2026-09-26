package genesisdb

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
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

func TestQueryIRUsesAPIKeyAndCapabilities(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer secret" {
			http.Error(w, `{"code":"AUTH_REQUIRED","message":"missing key"}`, http.StatusUnauthorized)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		if r.Method == http.MethodGet {
			_, _ = w.Write([]byte(`{"operations":{"context":"implemented"}}`))
			return
		}
		_, _ = w.Write([]byte(`{"operation_kind":"context","status":"ok"}`))
	}))
	defer server.Close()

	client := NewClientWithOptions(server.URL, ClientOptions{Timeout: 2 * time.Second, APIKey: "secret"})
	response, err := client.ExecuteQueryIR(context.Background(), map[string]interface{}{
		"contract_version": "query-ir.v1",
		"request_id":       "go-1",
		"operation": map[string]interface{}{
			"kind":      "context",
			"target_id": "n1",
			"tier":      "H0",
		},
	})
	if err != nil {
		t.Fatalf("ExecuteQueryIR returned error: %v", err)
	}
	if response["operation_kind"] != "context" {
		t.Fatalf("unexpected response: %#v", response)
	}

	capabilities, err := client.QueryIRCapabilities(context.Background())
	if err != nil {
		t.Fatalf("QueryIRCapabilities returned error: %v", err)
	}
	if capabilities["operations"].(map[string]interface{})["context"] != "implemented" {
		t.Fatalf("unexpected capabilities: %#v", capabilities)
	}
}

func TestQueryIRReturnsStructuredAPIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"code":"QUERY_CAPABILITY_UNSUPPORTED","message":"not implemented"}`))
	}))
	defer server.Close()

	_, err := NewClient(server.URL).ExecuteQueryIR(context.Background(), map[string]interface{}{})
	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected APIError, got %T: %v", err, err)
	}
	if apiErr.StatusCode != http.StatusBadRequest || apiErr.Code != "QUERY_CAPABILITY_UNSUPPORTED" {
		t.Fatalf("unexpected APIError: %#v", apiErr)
	}
}
