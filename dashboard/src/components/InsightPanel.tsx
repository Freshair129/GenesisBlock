import { useState } from 'react';
import { Activity, GitMerge, RefreshCcw, LayoutGrid, Share2 } from 'lucide-react';
import { useInsight } from '../hooks/useInsight';
import { CommunityGraph } from './CommunityGraph';

/**
 * GKS Insight panel: community clusters (size / impact / semantic drift) and the
 * structural gaps between them. Replaces the old "visualization coming soon"
 * placeholder. Reads /v1/insight/communities + /v1/insight/gaps and can trigger
 * a server-side recompute via /v1/insight/rebuild.
 */
export function InsightPanel() {
  const { graph, gaps, error, rebuilding, rebuild } = useInsight();
  const clusters = graph?.nodes ?? [];
  const maxMembers = Math.max(1, ...clusters.map((c) => c.member_count));
  const [view, setView] = useState<'cards' | 'graph'>('cards');

  return (
    <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-bold text-white flex items-center gap-2">
          <Activity size={18} className="text-accent-blue" /> GKS Insight
        </h2>
        <div className="flex items-center gap-2">
          {/* Cards / force-graph view toggle */}
          <div className="flex items-center rounded-lg border border-white/10 overflow-hidden">
            <button
              onClick={() => setView('cards')}
              title="Card list"
              className={`px-2 py-1.5 text-xs transition-colors ${
                view === 'cards' ? 'bg-white/10 text-white' : 'text-slate-400 hover:bg-white/5'
              }`}
            >
              <LayoutGrid size={14} />
            </button>
            <button
              onClick={() => setView('graph')}
              title="Force graph"
              className={`px-2 py-1.5 text-xs transition-colors ${
                view === 'graph' ? 'bg-white/10 text-white' : 'text-slate-400 hover:bg-white/5'
              }`}
            >
              <Share2 size={14} />
            </button>
          </div>
          <button
            onClick={() => rebuild()}
            disabled={rebuilding}
            className="flex items-center space-x-2 px-3 py-1.5 bg-background/50 hover:bg-white/5 rounded-lg text-xs font-medium border border-white/10 transition-colors disabled:opacity-50"
          >
            <RefreshCcw size={14} className={rebuilding ? 'animate-spin' : ''} />
            <span>{rebuilding ? 'Rebuilding…' : 'Rebuild'}</span>
          </button>
        </div>
      </div>

      {error && <div className="text-red-400 text-sm mb-3">{error}</div>}

      {clusters.length === 0 ? (
        <div className="text-slate-500 text-sm italic py-4">
          No communities yet — add vectors, then click{' '}
          <span className="text-slate-300">Rebuild</span> to detect clusters.
        </div>
      ) : view === 'graph' ? (
        <div className="space-y-2 mb-5">
          <h3 className="text-slate-400 text-xs uppercase tracking-wider">
            Communities ({clusters.length})
          </h3>
          <CommunityGraph nodes={clusters} edges={graph?.edges ?? []} gaps={gaps ?? []} />
        </div>
      ) : (
        <div className="space-y-2 mb-5">
          <h3 className="text-slate-400 text-xs uppercase tracking-wider">
            Communities ({clusters.length})
          </h3>
          {clusters.slice(0, 8).map((c) => (
            <div key={c.cluster_id} className="p-3 bg-background/50 rounded-lg border border-white/5">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-white truncate">
                  {c.theme || `Cluster ${c.cluster_id}`}
                </span>
                <span className="text-xs text-slate-400">{c.member_count} nodes</span>
              </div>
              <div className="mt-2 h-1.5 bg-white/5 rounded-full overflow-hidden">
                <div
                  className="h-full bg-accent-blue"
                  style={{ width: `${(c.member_count / maxMembers) * 100}%` }}
                />
              </div>
              <div className="mt-1 flex items-center gap-4 text-xs text-slate-500">
                <span>impact {c.impact.toFixed(2)}</span>
                {c.drift != null && <span>drift {c.drift.toFixed(3)}</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="space-y-2">
        <h3 className="text-slate-400 text-xs uppercase tracking-wider flex items-center gap-1">
          <GitMerge size={13} /> Structural gaps ({gaps?.length ?? 0})
        </h3>
        {gaps && gaps.length > 0 ? (
          gaps.slice(0, 6).map((g, i) => (
            <div
              key={`${g.cluster_a}-${g.cluster_b}-${i}`}
              className="flex items-center justify-between p-2 bg-background/50 rounded-lg border border-white/5 text-xs"
              title={g.reason}
            >
              <span className="text-slate-300">#{g.cluster_a} ↔ #{g.cluster_b}</span>
              <span className="text-accent-green">sim {g.similarity.toFixed(3)}</span>
            </div>
          ))
        ) : (
          <div className="text-slate-500 text-xs italic py-2">No structural gaps detected.</div>
        )}
      </div>
    </div>
  );
}

export default InsightPanel;
