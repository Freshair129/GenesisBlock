import { LayoutDashboard, Share2, Activity, Terminal, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

const NavItem = ({ icon: Icon, label, active = false }: { icon: LucideIcon, label: string, active?: boolean }) => (
  <div className={`flex items-center space-x-3 p-3 rounded-lg cursor-pointer transition-colors ${active ? 'bg-surface text-accent-blue' : 'hover:bg-surface/50'}`}>
    <Icon size={20} />
    <span className="font-medium">{label}</span>
  </div>
);

export const Sidebar = () => {
  return (
    <aside className="w-64 border-r border-white/10 p-4 flex flex-col space-y-6">
      <div className="flex items-center space-x-2 px-2 py-4">
        <div className="w-8 h-8 bg-accent-blue rounded flex items-center justify-center font-bold text-background">G</div>
        <span className="text-xl font-bold tracking-tight text-white">GenesisBlock</span>
      </div>

      <nav className="flex-1 space-y-2">
        <NavItem icon={LayoutDashboard} label="Status Hub" active />
        <NavItem icon={Share2} label="Swarm Swarm" />
        <NavItem icon={Activity} label="Graph Navigator" />
        <NavItem icon={Terminal} label="HQL Console" />
      </nav>

      <div className="pt-4 border-t border-white/10">
        <NavItem icon={Settings} label="Settings" />
      </div>
    </aside>
  );
};
