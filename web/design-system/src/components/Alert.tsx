import React from 'react';

export type AlertType = 'info' | 'success' | 'warning' | 'error';

export interface AlertProps {
  type?: AlertType;
  title: string;
  description?: string;
  icon?: string;
}

const alertConfig: Record<AlertType, { bg: string; border: string; icon: string }> = {
  info: { bg: 'bg-blue/15', border: 'border-blue/30', icon: 'ℹ️' },
  success: { bg: 'bg-green/15', border: 'border-green/30', icon: '✅' },
  warning: { bg: 'bg-yellow/15', border: 'border-yellow/30', icon: '⚠️' },
  error: { bg: 'bg-red/15', border: 'border-red/30', icon: '❌' },
};

export function Alert({ type = 'info', title, description, icon }: AlertProps) {
  const config = alertConfig[type];
  const displayIcon = icon || config.icon;

  return (
    <div className={`rounded-lg p-3 flex gap-2.5 items-start ${config.bg} border ${config.border}`}>
      <div className="text-base leading-none flex-shrink-0">{displayIcon}</div>
      <div className="flex-1 min-w-0">
        <div className="text-xs font-semibold text-white">{title}</div>
        {description && (
          <div className="text-[11px] text-zinc-400 mt-0.5">{description}</div>
        )}
      </div>
    </div>
  );
}

export default Alert;
