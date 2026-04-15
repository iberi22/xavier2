import React from 'react';

export interface StatsRowProps {
  stats: Array<{
    label: string;
    value: string | number;
    icon?: string;
  }>;
}

export function StatsRow({ stats }: StatsRowProps) {
  return (
    <div className="grid gap-2" style={{ gridTemplateColumns: `repeat(${Math.min(stats.length, 4)}, 1fr)` }}>
      {stats.map((stat, i) => (
        <div key={i} className="bg-bg-3 rounded-lg p-3 text-center">
          {stat.icon && <div className="text-lg mb-1">{stat.icon}</div>}
          <div className="text-xl font-bold text-white">{stat.value}</div>
          <div className="text-[9px] text-zinc-500 uppercase mt-1">{stat.label}</div>
        </div>
      ))}
    </div>
  );
}

export default StatsRow;
