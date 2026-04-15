import React from 'react';

export interface ChecklistItem {
  label: string;
  done: boolean;
}

export interface ChecklistProps {
  items?: ChecklistItem[];
}

export function Checklist({ items }: ChecklistProps) {
  const defaultItems: ChecklistItem[] = [
    { label: 'Memory manager implemented', done: true },
    { label: 'E2E encryption designed', done: true },
    { label: 'Cloud deployment', done: false },
    { label: 'Mobile app', done: false },
  ];

  const checkItems = items || defaultItems;

  return (
    <div className="bg-bg-3 rounded-lg p-2.5">
      {checkItems.map((item, i) => (
        <div
          key={i}
          className="flex items-center gap-2 py-1.5 border-b border-border last:border-b-0 text-xs"
          style={{ borderColor: '#3f3f46' }}
        >
          <div
            className={`w-4 h-4 rounded-full border-2 flex items-center justify-center text-[9px] flex-shrink-0 ${
              item.done ? 'bg-green border-green' : 'border-zinc-600'
            }`}
          >
            {item.done && '✓'}
          </div>
          <span className={item.done ? 'line-through text-zinc-500' : 'text-zinc-200'}>
            {item.label}
          </span>
        </div>
      ))}
    </div>
  );
}

export default Checklist;
