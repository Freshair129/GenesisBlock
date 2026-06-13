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
};

export default api;
