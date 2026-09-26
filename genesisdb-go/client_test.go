package genesisdb

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

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
