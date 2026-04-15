import React from 'react';

export interface MinilistItem {
  label: string;
  value: string | number;
}

export interface MinilistProps {
  items?: MinilistItem[];
}

export function Minilist({ items }: MinilistProps) {
  const defaultItems: MinilistItem[] = [
    { label: 'Memory', value: '415' },
    { label: 'Recall', value: '99.1%' },
    { label: 'Precision', value: '4.45/5' },
    { label: 'Latency', value: '56ms' },
  ];

  const listItems = items || defaultItems;

  return (
    <div className="flex flex-col gap-1">
      {listItems.map((item, i) => (
        <div
          key={i}
          className="flex justify-between px-2.5 py-1.5 bg-bg-3 rounded text-[11px]"
        >
          <span className="font-semibold text-zinc-300">{item.label}</span>
          <span className="text-white font-medium">{item.value}</span>
        </div>
      ))}
    </div>
  );
}

export default Minilist;
