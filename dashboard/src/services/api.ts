import axios from 'axios';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

const client = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export interface ExtendedStatus {
  open: boolean;
  read_only: boolean;
  page_cache_mb: number;
  node_count: number;
  edge_count: number;
  memory_usage_mb: number;
}

export interface SyncPeer {
  id: string;
  addr: string;
  last_seen: number;
  verifying_key: number[];
}

export interface SwarmStatus {
  peer_id: string;
  logical_clock: number;
  peers: SyncPeer[];
}

// --- GKS Insight (community clusters / structural gaps) ---

export interface SuperNode {
  cluster_id: number;
  theme: string;
  member_count: number;
  impact: number;
  centroid: number[];
  timestamp: string;
  drift: number | null;
}

export interface MetaEdge {
  from_cluster: number;
  to_cluster: number;
  weight: number;
}

export interface CommunityGraph {
  nodes: SuperNode[];
  edges: MetaEdge[];
}

export interface GapSuggestion {
  cluster_a: number;
  cluster_b: number;
  similarity: number;
  reason: string;
}

export const api = {
  getStatus: async () => {
    const response = await client.get<ExtendedStatus>('/v1/status');
    return response.data;
  },

  getSwarmStatus: async () => {
    const response = await client.get<SwarmStatus>('/v1/swarm/status');
    return response.data;
  },

  executeHql: async (query: string) => {
    const response = await client.post('/v1/query/hql', JSON.stringify(query));
    return response.data;
  },

  getCommunities: async () => {
    const response = await client.get<CommunityGraph>('/v1/insight/communities');
    return response.data;
  },

  getGaps: async () => {
    const response = await client.get<GapSuggestion[]>('/v1/insight/gaps');
    return response.data;
  },

  rebuildInsight: async () => {
    const response = await client.post('/v1/insight/rebuild');
    return response.data;
  },
};

export default api;
