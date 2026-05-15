import React from 'react';

interface IconProps {
  name: string;
}

export const Icon: React.FC<IconProps> = ({ name }) => {
  const common = { 
    width: 16, 
    height: 16, 
    fill: "none", 
    stroke: "currentColor", 
    strokeWidth: 1.8, 
    strokeLinecap: "round" as const, 
    strokeLinejoin: "round" as const 
  };

  if (name === "layers") return <svg viewBox="0 0 24 24" {...common}><path d="M12 3l9 5-9 5-9-5 9-5z"/><path d="M3 13l9 5 9-5"/></svg>;
  if (name === "hash")   return <svg viewBox="0 0 24 24" {...common}><line x1="4" y1="9" x2="20" y2="9"/><line x1="4" y1="15" x2="20" y2="15"/><line x1="10" y1="3" x2="8" y2="21"/><line x1="16" y1="3" x2="14" y2="21"/></svg>;
  if (name === "calendar") return <svg viewBox="0 0 24 24" {...common}><rect x="3" y="4" width="18" height="18" rx="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>;
  if (name === "settings") return <svg viewBox="0 0 24 24" {...common}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>;
  if (name === "graph")  return <svg viewBox="0 0 24 24" {...common}><circle cx="6" cy="6" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="12" cy="18" r="2"/><line x1="7.4" y1="7.4" x2="10.6" y2="16.6"/><line x1="16.6" y1="7.4" x2="13.4" y2="16.6"/><line x1="8" y1="6" x2="16" y2="6"/></svg>;
  if (name === "note")   return <svg viewBox="0 0 24 24" {...common}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>;
  if (name === "embed")  return <svg viewBox="0 0 24 24" {...common}><circle cx="6" cy="6" r="1.6"/><circle cx="14" cy="9" r="1.6"/><circle cx="9" cy="15" r="1.6"/><circle cx="18" cy="17" r="1.6"/><circle cx="20" cy="7" r="1.6"/></svg>;
  if (name === "chat")   return <svg viewBox="0 0 24 24" {...common}><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>;
  if (name === "search") return <svg viewBox="0 0 24 24" {...common}><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>;
  if (name === "x")      return <svg viewBox="0 0 24 24" {...common}><line x1="6" y1="6" x2="18" y2="18"/><line x1="6" y1="18" x2="18" y2="6"/></svg>;
  if (name === "menu")   return <svg viewBox="0 0 24 24" {...common}><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>;
  if (name === "info")   return <svg viewBox="0 0 24 24" {...common}><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>;
  return null;
};
