import React from 'react';

export interface MetricBigProps {
  label: string;
  value: string | number;
  icon?: string;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
}

export function MetricBig({ label, value, icon, trend, trendValue }: MetricBigProps) {
  return (
    <div className="text-center p-6 bg-gradient-to-br from-bg-3 to-bg-2 rounded-xl">
      {icon && <div className="text-3xl mb-2">{icon}</div>}
      <div
        className="text-5xl font-bold bg-gradient-to-r from-accent to-purple bg-clip-text text-transparent"
        style={{ WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}
      >
        {value}
      </div>
      <div className="text-xs text-zinc-400 uppercase tracking-widest mt-2">{label}</div>
      {trend && trendValue && (
        <span
          className={`inline-block mt-2 px-2 py-0.5 rounded-full text-xs font-medium ${
            trend === 'up'
              ? 'bg-green/20 text-green'
              : trend === 'down'
              ? 'bg-red/20 text-red'
              : 'bg-zinc-700 text-zinc-400'
          }`}
        >
          {trend === 'up' ? '↑' : trend === 'down' ? '↓' : '→'} {trendValue}
        </span>
      )}
    </div>
  );
}

export default MetricBig;
