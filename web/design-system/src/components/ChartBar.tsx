import React from 'react';

export interface ChartBarProps {
  data: Array<{
    label: string;
    value: number;
    color?: string;
  }>;
  title?: string;
}

export function ChartBar({ data, title }: ChartBarProps) {
  const maxValue = Math.max(...data.map((d) => d.value), 1);

  return (
    <div className="bg-bg-3 rounded-lg p-4">
      {title && <div className="text-xs font-semibold mb-3 text-white">{title}</div>}
      <div className="flex items-end gap-1.5 h-20">
        {data.map((item, i) => (
          <div key={i} className="flex-1 flex flex-col items-center gap-1">
            <div
              className="w-full rounded-t min-h-1"
              style={{
                height: `${Math.max((item.value / maxValue) * 80, 4)}px`,
                background: item.color || 'linear-gradient(180deg, #6366f1, #4f46e5)',
              }}
            />
            <div className="text-[9px] text-zinc-500">{item.label}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default ChartBar;
