import React from 'react';

export interface IconStatProps {
  icon: string;
  value: string | number;
  label: string;
  color?: string;
}

export function IconStat({ icon, value, label, color = '#6366f1' }: IconStatProps) {
  return (
    <div className="bg-bg-3 rounded-lg p-3.5 flex items-center gap-3">
      <div
        className="w-10 h-10 rounded-xl flex items-center justify-center text-xl flex-shrink-0"
        style={{ background: color + '30' }}
      >
        {icon}
      </div>
      <div>
        <div className="text-xl font-bold" style={{ color }}>
          {value}
        </div>
        <div className="text-[10px] text-zinc-500 mt-0.5">{label}</div>
      </div>
    </div>
  );
}

export default IconStat;
