import { Bell, Cpu, Wifi } from 'lucide-react';

export const Header = () => {
  return (
    <header className="h-16 border-b border-white/10 px-6 flex items-center justify-between">
      <div className="text-sm text-slate-500 font-mono">MARK XII // COGNITIVE_RETRIEVAL_ENGINE</div>

      <div className="flex items-center space-x-4">
        <div className="flex items-center space-x-2 px-3 py-1 bg-accent-green/10 text-accent-green rounded-full text-xs font-medium border border-accent-green/20">
          <Wifi size={14} />
          <span>SWARM ONLINE</span>
        </div>
        <div className="flex items-center space-x-2 px-3 py-1 bg-accent-blue/10 text-accent-blue rounded-full text-xs font-medium border border-accent-blue/20">
          <Cpu size={14} />
          <span>ENGINE READY</span>
        </div>
        <button className="p-2 hover:bg-white/5 rounded-full transition-colors">
          <Bell size={18} />
        </button>
      </div>
    </header>
  );
};
