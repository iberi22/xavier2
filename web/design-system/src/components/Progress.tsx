import React from 'react';

export interface ProgressItem {
  label: string;
  value: number;
  color?: string;
}

export interface ProgressProps {
  items?: ProgressItem[];
}

export function Progress({ items }: ProgressProps) {
  const defaultItems: ProgressItem[] = [
    { label: 'Development', value: 85, color: '#6366f1' },
    { label: 'Testing', value: 60, color: '#eab308' },
    { label: 'Documentation', value: 40, color: '#22c55e' },
  ];

  const progressItems = items || defaultItems;

  return (
    <div className="flex flex-col gap-2.5">
      {progressItems.map((item, i) => (
        <div key={i} className="bg-bg-3 rounded-md p-3">
          <div className="flex justify-between text-[11px] mb-2">
            <span className="text-zinc-300">{item.label}</span>
            <span className="text-white font-medium">{item.value}%</span>
          </div>
          <div className="h-1.5 bg-bg rounded overflow-hidden">
            <div
              className="h-full rounded"
              style={{
                width: `${item.value}%`,
                background: item.color || '#6366f1',
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

export default Progress;
