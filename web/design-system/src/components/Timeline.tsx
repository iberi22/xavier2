import React from 'react';

export interface TimelineItem {
  date: string;
  title: string;
  description?: string;
  color?: string;
}

export interface TimelineProps {
  items?: TimelineItem[];
}

export function Timeline({ items }: TimelineProps) {
  const defaultItems: TimelineItem[] = [
    { date: 'Apr 5, 2026', title: 'Xavier 99.1% recall', description: 'LoCoMo benchmark' },
    { date: 'Apr 4, 2026', title: 'Memory manager', description: 'Decay+consolidate+evict' },
    { date: 'Apr 3, 2026', title: 'SurrealDB v3', description: 'Upgrade complete' },
  ];

  const timelineItems = items || defaultItems;

  return (
    <div className="relative pl-4">
      <div
        className="absolute left-1.5 top-1.5 bottom-1.5 w-0.5 bg-border"
        style={{ background: '#3f3f46' }}
      />
      {timelineItems.map((item, i) => (
        <div key={i} className="relative pb-3.5 last:pb-0">
          <div
            className="absolute w-2 h-2 rounded-full -left-1 top-1"
            style={{ background: item.color || '#6366f1' }}
          />
          <div className="text-[10px] text-zinc-500 mb-0.5">{item.date}</div>
          <div className="text-xs font-semibold text-white">{item.title}</div>
          {item.description && (
            <div className="text-[11px] text-zinc-400 mt-0.5">{item.description}</div>
          )}
        </div>
      ))}
    </div>
  );
}

export default Timeline;
