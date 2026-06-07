package genesisdb

// Node represents a GKS knowledge atom.
type Node struct {
	ID        string                 `json:"id"`
	Labels    []string               `json:"labels"`
	Props     map[string]interface{} `json:"props"`
	Impact    float64                `json:"impact,omitempty"`
	Lang      string                 `json:"lang,omitempty"`
	ExpiresAt string                 `json:"expires_at,omitempty"`
}

// Edge represents a relationship between two nodes.
type Edge struct {
	ID    string                 `json:"id"`
	From  string                 `json:"from"`
	To    string                 `json:"to"`
	Rel   string                 `json:"rel"`
	Props map[string]interface{} `json:"props"`
}

// ContextPackage represents a tiered knowledge fragment.
type ContextPackage struct {
	Nodes          []Node                   `json:"nodes"`
	Edges          []Edge                   `json:"edges"`
	SuperNodes     []map[string]interface{} `json:"super_nodes"`
	TokenEstimate  uint32                   `json:"token_estimate"`
	ReasoningPath  string                   `json:"reasoning_path"`
}

// NodeInput is used for creating new nodes.
type NodeInput struct {
	ID        string                 `json:"id,omitempty"`
	Labels    []string               `json:"labels"`
	Props     map[string]interface{} `json:"props,omitempty"`
	Embedding []float64              `json:"embedding,omitempty"`
	Lang      string                 `json:"lang,omitempty"`
	TTL       uint32                 `json:"ttl,omitempty"`
	CausedBy  string                 `json:"caused_by,omitempty"`
}
