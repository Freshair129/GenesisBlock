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
}

// NewClient creates a new GenesisDB client.
func NewClient(baseURL string) *Client {
	return &Client{
		BaseURL: strings.TrimSuffix(baseURL, "/"),
		HTTPClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
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

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		var errBuf bytes.Buffer
		errBuf.ReadFrom(resp.Body)
		return nil, fmt.Errorf("server error (%d): %s", resp.StatusCode, errBuf.String())
	}

	var resBuf bytes.Buffer
	resBuf.ReadFrom(resp.Body)
	return resBuf.Bytes(), nil
}
