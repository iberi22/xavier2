import React from 'react';

export interface LeaderboardItem {
  name: string;
  value: string | number;
  rank?: number;
}

export interface LeaderboardProps {
  items?: LeaderboardItem[];
}

const rankColors = ['text-amber-400', 'text-zinc-400', 'text-amber-600'];

export function Leaderboard({ items }: LeaderboardProps) {
  return (
    <div className="bg-bg-3 rounded-lg overflow-hidden">
      {(items || [
        { name: 'ManteniApp', value: '100' },
        { name: 'Xavier2', value: '95' },
        { name: 'Synapse', value: '87' },
        { name: 'Edge Hive', value: '72' },
      ]).map((item, i) => (
        <div
          key={i}
          className="grid grid-cols-[30px_1fr_50px] px-3 py-2 text-xs items-center border-b border-border last:border-b-0"
          style={{ borderColor: '#3f3f46' }}
        >
          <span className={`font-bold ${i < 3 ? rankColors[i] : 'text-zinc-500'}`}>
            {i + 1}
          </span>
          <span className="truncate text-zinc-200">{item.name}</span>
          <span className="text-right font-semibold text-white">{item.value}</span>
        </div>
      ))}
    </div>
  );
}

export default Leaderboard;
