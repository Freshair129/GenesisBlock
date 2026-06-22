import { useCallback, useEffect, useState } from 'react';
import api from '../services/api';
import type { CommunityGraph, GapSuggestion } from '../services/api';

/**
 * GKS Insight: community clusters (meta-graph) + structural gaps. Polls the
 * read-only views; `rebuild()` triggers a server-side recompute then refetches.
 */
export const useInsight = () => {
  const [graph, setGraph] = useState<CommunityGraph | null>(null);
  const [gaps, setGaps] = useState<GapSuggestion[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rebuilding, setRebuilding] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const [g, gp] = await Promise.all([api.getCommunities(), api.getGaps()]);
      setGraph(g);
      setGaps(gp);
      setError(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to fetch insight');
    }
  }, []);

  const rebuild = useCallback(async () => {
    setRebuilding(true);
    try {
      await api.rebuildInsight();
      await fetchData();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Insight rebuild failed');
    } finally {
      setRebuilding(false);
    }
  }, [fetchData]);

  useEffect(() => {
    const initial = setTimeout(fetchData, 0);
    const interval = setInterval(fetchData, 10000); // Poll every 10s
    return () => {
      clearTimeout(initial);
      clearInterval(interval);
    };
  }, [fetchData]);

  return { graph, gaps, error, rebuilding, rebuild, refresh: fetchData };
};
