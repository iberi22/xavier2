import React from 'react';

export interface BadgeRowProps {
  badges?: Array<{
    label: string;
    variant?: 'blue' | 'green' | 'yellow' | 'red' | 'purple';
  }>;
}

const variantConfig = {
  blue: 'bg-blue/20 text-blue',
  green: 'bg-green/20 text-green',
  yellow: 'bg-yellow/20 text-yellow',
  red: 'bg-red/20 text-red',
  purple: 'bg-purple/20 text-purple',
};

export function BadgeRow({ badges }: BadgeRowProps) {
  const defaultBadges = [
    { label: 'Xavier2', variant: 'blue' as const },
    { label: 'Memory', variant: 'green' as const },
    { label: 'AI', variant: 'purple' as const },
    { label: 'Secure', variant: 'yellow' as const },
    { label: 'Fast', variant: 'red' as const },
  ];

  const badgeItems = badges || defaultBadges;

  return (
    <div className="flex flex-wrap gap-1.5">
      {badgeItems.map((badge, i) => (
        <span
          key={i}
          className={`px-2.5 py-1 rounded-full text-[11px] font-medium ${variantConfig[badge.variant || 'blue']}`}
        >
          {badge.label}
        </span>
      ))}
    </div>
  );
}

export default BadgeRow;
