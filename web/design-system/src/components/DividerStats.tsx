import React from 'react';

export interface DividerStatsProps {
  left: { value: string | number; label: string; color?: string };
  right: { value: string | number; label: string; color?: string };
}

export function DividerStats({ left, right }: DividerStatsProps) {
  return (
    <div
      className="grid grid-cols-[1fr_1px_1fr] gap-4 items-center bg-bg-3 rounded-lg p-4"
    >
      <div className="text-center">
        <div
          className="text-2xl font-bold"
          style={{ color: left.color || '#6366f1' }}
        >
          {left.value}
        </div>
        <div className="text-[10px] text-zinc-500 mt-1">{left.label}</div>
      </div>
      <div className="h-full w-px bg-border" style={{ background: '#3f3f46' }} />
      <div className="text-center">
        <div
          className="text-2xl font-bold"
          style={{ color: right.color || '#22c55e' }}
        >
          {right.value}
        </div>
        <div className="text-[10px] text-zinc-500 mt-1">{right.label}</div>
      </div>
    </div>
  );
}

export default DividerStats;
