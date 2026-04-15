import React from 'react';

export interface HeroBannerProps {
  value: string;
  label: string;
  subtitle?: string;
  gradientFrom?: string;
  gradientTo?: string;
}

export function HeroBanner({ value, label, subtitle, gradientFrom = '#6366f1', gradientTo = '#a855f7' }: HeroBannerProps) {
  return (
    <div
      className="rounded-xl p-8 text-center relative overflow-hidden"
      style={{
        background: `linear-gradient(135deg, ${gradientFrom}, ${gradientTo})`,
      }}
    >
      <div className="relative z-10">
        <div className="text-5xl font-bold text-white mb-1">{value}</div>
        <div className="text-base text-white/90">{label}</div>
        {subtitle && <div className="text-xs text-white/60 mt-2">{subtitle}</div>}
      </div>
    </div>
  );
}

export default HeroBanner;
