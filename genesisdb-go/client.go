package genesisdb

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"
)

// Client is a Go client for GenesisBlock DB.
type Client struct {
	BaseURL    string
	HTTPClient *http.Client
	APIKey     string
}

// ClientOptions configures transport behavior without changing NewClient's
// backwards-compatible constructor.
type ClientOptions struct {
	HTTPClient *http.Client
	Timeout    time.Duration
	APIKey     string
}

// NewClient creates a new GenesisBlockDB client.
func NewClient(baseURL string) *Client {
	return NewClientWithOptions(baseURL, ClientOptions{})
}

// NewClientWithOptions creates a client with a finite timeout and optional
// bearer API key. A custom HTTP client takes precedence over Timeout.
func NewClientWithOptions(baseURL string, options ClientOptions) *Client {
	httpClient := options.HTTPClient
	if httpClient == nil {
		timeout := options.Timeout
		if timeout <= 0 {
			timeout = 30 * time.Second
		}
		httpClient = &http.Client{Timeout: timeout}
	}
	return &Client{
		BaseURL: strings.TrimSuffix(baseURL, "/"),
		HTTPClient: httpClient,
		APIKey:     options.APIKey,
	}
}

// APIError is a structured error returned by a GenesisBlockDB REST route.
type APIError struct {
	StatusCode int
	Code       string
	Message    string
}

func (e *APIError) Error() string {
	return fmt.Sprintf("server error (%d) %s: %s", e.StatusCode, e.Code, e.Message)
}

// Query executes a raw HQL command.
func (c *Client) Query(ctx context.Context, hql string) (interface{}, error) {
	url := fmt.Sprintf("%s/v1/query/hql", c.BaseURL)
	payload := map[string]string{"query": hql}
	
	res, err := c.post(ctx, url, payload)
	if err != nil {
		return nil, err
	}
	
	var result interface{}
	if err := json.Unmarshal(res, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal HQL result: %w", err)
	}
	return result, nil
}

// AddNode adds a new node to the graph.
func (c *Client) AddNode(ctx context.Context, input NodeInput) (*Node, error) {
	url := fmt.Sprintf("%s/v1/node/add", c.BaseURL)
	
	if input.CausedBy == "" {
		input.CausedBy = "go-sdk"
	}

	res, err := c.post(ctx, url, input)
	if err != nil {
		return nil, err
	}
	
	var node Node
	if err := json.Unmarshal(res, &node); err != nil {
		return nil, fmt.Errorf("failed to unmarshal Node result: %w", err)
	}
	return &node, nil
}

// GetContext retrieves a tiered knowledge fragment.
func (c *Client) GetContext(ctx context.Context, target string, tier string, budget *uint32) (*ContextPackage, error) {
	hql := fmt.Sprintf("CONTEXT FOR %s TIER %s", target, tier)
	if budget != nil {
		hql = fmt.Sprintf("%s BUDGET %d", hql, *budget)
	}
	
	res, err := c.Query(ctx, hql)
	if err != nil {
		return nil, err
	}
	
	// Re-marshal/unmarshal to get typed ContextPackage
	data, _ := json.Marshal(res)
	var pkg ContextPackage
	if err := json.Unmarshal(data, &pkg); err != nil {
		return nil, fmt.Errorf("failed to parse ContextPackage: %w", err)
	}
	return &pkg, nil
}

// ExecuteQueryIR executes a closed, versioned Query IR request.
func (c *Client) ExecuteQueryIR(ctx context.Context, request map[string]interface{}) (map[string]interface{}, error) {
	url := fmt.Sprintf("%s/v1/query/ir", c.BaseURL)
	res, err := c.post(ctx, url, request)
	if err != nil {
		return nil, err
	}
	var response map[string]interface{}
	if err := json.Unmarshal(res, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal Query IR response: %w", err)
	}
	return response, nil
}

// QueryIRCapabilities returns the server's operation and boundary manifest.
func (c *Client) QueryIRCapabilities(ctx context.Context) (map[string]interface{}, error) {
	url := fmt.Sprintf("%s/v1/query/ir/capabilities", c.BaseURL)
	res, err := c.get(ctx, url)
	if err != nil {
		return nil, err
	}
	var response map[string]interface{}
	if err := json.Unmarshal(res, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal Query IR capabilities: %w", err)
	}
	return response, nil
}

func (c *Client) post(ctx context.Context, url string, payload interface{}) ([]byte, error) {
	body, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal payload: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewBuffer(body))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if c.APIKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.APIKey)
	}

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		var errBuf bytes.Buffer
		errBuf.ReadFrom(resp.Body)
		message := errBuf.String()
		var envelope struct {
			Code    string `json:"code"`
			Message string `json:"message"`
		}
		if json.Unmarshal(errBuf.Bytes(), &envelope) == nil && envelope.Message != "" {
			return nil, &APIError{StatusCode: resp.StatusCode, Code: envelope.Code, Message: envelope.Message}
		}
		return nil, &APIError{StatusCode: resp.StatusCode, Code: "QUERY_EXECUTION_FAILED", Message: message}
	}

	var resBuf bytes.Buffer
	resBuf.ReadFrom(resp.Body)
	return resBuf.Bytes(), nil
}

func (c *Client) get(ctx context.Context, url string) ([]byte, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if c.APIKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.APIKey)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		var errBuf bytes.Buffer
		errBuf.ReadFrom(resp.Body)
		var envelope struct {
			Code    string `json:"code"`
			Message string `json:"message"`
		}
		if json.Unmarshal(errBuf.Bytes(), &envelope) == nil && envelope.Message != "" {
			return nil, &APIError{StatusCode: resp.StatusCode, Code: envelope.Code, Message: envelope.Message}
		}
		return nil, &APIError{StatusCode: resp.StatusCode, Code: "QUERY_EXECUTION_FAILED", Message: errBuf.String()}
	}
	var resBuf bytes.Buffer
	resBuf.ReadFrom(resp.Body)
	return resBuf.Bytes(), nil
}
