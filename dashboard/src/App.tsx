import { Layout } from './components/Layout';
import { useStatus } from './hooks/useStatus';
import { RefreshCcw, AlertTriangle } from 'lucide-react';

function App() {
  const { status, swarm, loading, error, refresh } = useStatus();

  return (
    <Layout>
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-white">System Overview</h1>
          <button
            onClick={() => refresh()}
            className="flex items-center space-x-2 px-3 py-1.5 bg-surface hover:bg-white/5 rounded-lg text-xs font-medium border border-white/10 transition-colors"
          >
            <RefreshCcw size={14} className={loading ? 'animate-spin' : ''} />
            <span>Refresh</span>
          </button>
        </div>

        {error && (
          <div className="bg-red-500/10 border border-red-500/20 p-4 rounded-xl flex items-center space-x-3 text-red-400">
            <AlertTriangle size={20} />
            <span className="text-sm font-medium">Backend connection failed: {error}</span>
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg">
            <h3 className="text-slate-400 text-sm font-medium mb-1 uppercase tracking-wider">Local Peer ID</h3>
            <p className="text-lg font-mono text-accent-blue truncate">
              {swarm?.peer_id || 'Connecting...'}
            </p>
          </div>
          <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg">
            <h3 className="text-slate-400 text-sm font-medium mb-1 uppercase tracking-wider">Logical Clock</h3>
            <p className="text-lg font-mono text-white">
              {swarm?.logical_clock.toLocaleString() ?? '---'}
            </p>
          </div>
          <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg">
            <h3 className="text-slate-400 text-sm font-medium mb-1 uppercase tracking-wider">Node Count</h3>
            <p className="text-lg font-mono text-white">
              {status?.node_count.toLocaleString() ?? '---'}
            </p>
          </div>
          <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg">
            <h3 className="text-slate-400 text-sm font-medium mb-1 uppercase tracking-wider">Memory Usage</h3>
            <p className="text-lg font-mono text-white">
              {status ? `${status.memory_usage_mb.toFixed(2)} MB` : '---'}
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="bg-surface border border-white/5 p-6 rounded-xl shadow-lg">
            <h2 className="text-lg font-bold text-white mb-4">Swarm Peers ({swarm?.peers.length ?? 0})</h2>
            {swarm && swarm.peers.length > 0 ? (
              <div className="space-y-3">
                {swarm.peers.map((peer) => (
                  <div key={peer.id} className="flex items-center justify-between p-3 bg-background/50 rounded-lg border border-white/5">
                    <div className="flex flex-col">
                      <span className="text-sm font-mono text-accent-blue">{peer.id}</span>
                      <span className="text-xs text-slate-500">{peer.addr}</span>
                    </div>
                    <div className="text-xs text-accent-green">Online</div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-slate-500 text-sm italic py-4">No remote peers discovered yet.</div>
            )}
          </div>

          <div className="bg-surface border border-white/5 p-8 rounded-xl h-full flex flex-col">
             <h2 className="text-lg font-bold text-white mb-4">Visual Insight Preview</h2>
             <div className="flex-1 flex items-center justify-center text-slate-500 italic text-center">
              Graph visualization will be implemented in Phase 3
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default App;
