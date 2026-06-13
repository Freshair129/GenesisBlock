import { useCallback, useEffect, useState } from 'react';
import api from '../services/api';
import type { ExtendedStatus, SwarmStatus } from '../services/api';

export const useStatus = () => {
  const [status, setStatus] = useState<ExtendedStatus | null>(null);
  const [swarm, setSwarm] = useState<SwarmStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [statusData, swarmData] = await Promise.all([
        api.getStatus(),
        api.getSwarmStatus(),
      ]);
      setStatus(statusData);
      setSwarm(swarmData);
      setError(null);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Failed to fetch system status';
      setError(message);
    }
  }, []);

  useEffect(() => {
    const initialFetch = setTimeout(fetchData, 0);
    const interval = setInterval(fetchData, 5000); // Poll every 5 seconds
    return () => {
      clearTimeout(initialFetch);
      clearInterval(interval);
    };
  }, [fetchData]);

  const loading = status === null && swarm === null && error === null;
  return { status, swarm, loading, error, refresh: fetchData };
};
