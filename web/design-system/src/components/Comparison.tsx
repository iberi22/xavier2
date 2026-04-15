import React from 'react';

export interface ComparisonProps {
  left: { label: string; value: string | number };
  right: { label: string; value: string | number };
  vs?: string;
}

export function Comparison({ left, right, vs = 'vs' }: ComparisonProps) {
  return (
    <div className="grid grid-cols-[1fr_auto_1fr] gap-2.5 items-center">
      <div className="bg-bg-3 rounded-lg p-3 text-center">
        <div className="text-[10px] uppercase text-zinc-500 mb-1.5">{left.label}</div>
        <div className="text-xl font-bold text-white">{left.value}</div>
      </div>
      <div className="text-[11px] text-zinc-500 font-semibold px-1">{vs}</div>
      <div className="bg-bg-3 rounded-lg p-3 text-center">
        <div className="text-[10px] uppercase text-zinc-500 mb-1.5">{right.label}</div>
        <div className="text-xl font-bold text-white">{right.value}</div>
      </div>
    </div>
  );
}

export default Comparison;
