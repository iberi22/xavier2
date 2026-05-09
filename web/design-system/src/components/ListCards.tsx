import React from 'react';

export interface ListCardItem {
  icon?: string;
  title: string;
  subtitle?: string;
  value?: string | number;
  color?: string;
}

export interface ListCardsProps {
  items?: ListCardItem[];
}

export function ListCards({ items }: ListCardsProps) {
  const defaultItems: ListCardItem[] = [
    { icon: '📊', title: 'Xavier Memory', subtitle: 'AI-powered search' },
    { icon: '🔒', title: 'E2E Encryption', subtitle: 'AES-256-GCM' },
    { icon: '🛡️', title: 'Anticipator', subtitle: 'Prompt injection protection' },
  ];

  const listItems = items || defaultItems;

  return (
    <div className="flex flex-col gap-1.5">
      {listItems.map((item, i) => (
        <div key={i} className="bg-bg-3 rounded-md p-3 flex items-center gap-3">
          {item.icon && (
            <div
              className="w-8 h-8 rounded-lg flex items-center justify-center text-sm flex-shrink-0"
              style={{ background: (item.color as string) || '#6366f1' + '20' }}
            >
              {item.icon}
            </div>
          )}
          <div className="flex-1 min-w-0">
            <div className="text-xs font-medium text-white truncate">{item.title}</div>
            {item.subtitle && (
              <div className="text-[10px] text-zinc-500 truncate">{item.subtitle}</div>
            )}
          </div>
          {item.value !== undefined && (
            <div className="text-xs font-semibold text-accent flex-shrink-0">{item.value}</div>
          )}
        </div>
      ))}
    </div>
  );
}

export default ListCards;
