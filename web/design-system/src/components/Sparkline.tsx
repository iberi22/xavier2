import React from 'react';

export interface SparklineProps {
  value?: string;
  label?: string;
  data?: number[];
  color?: string;
}

export function Sparkline({ value = '+12%', label = 'Trend', data, color = '#22c55e' }: SparklineProps) {
  const defaultData = [30, 45, 35, 60, 50, 70, 65, 80];
  const chartData = data || defaultData;
  const max = Math.max(...chartData, 1);

  return (
    <div className="bg-bg-3 rounded-lg p-3">
      <div className="flex justify-between items-center mb-2">
        <span className="text-[11px] font-semibold text-zinc-300">{label}</span>
        <span className="text-sm font-bold" style={{ color }}>
          {value}
        </span>
      </div>
      <div className="flex gap-0.5 h-8 items-end">
        {chartData.map((d, i) => (
          <div
            key={i}
            className="flex-1 rounded-sm min-h-1"
            style={{
              height: `${Math.max((d / max) * 100, 10)}%`,
              background: color,
            }}
          />
        ))}
      </div>
    </div>
  );
}

export default Sparkline;
