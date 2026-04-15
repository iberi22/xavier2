import React from 'react';

export interface PipelineStage {
  name: string;
  count: number;
  color?: string;
}

export interface PipelineProps {
  stages?: PipelineStage[];
}

export function Pipeline({ stages }: PipelineProps) {
  const defaultStages: PipelineStage[] = [
    { name: 'Lead', count: 120, color: '#3b82f6' },
    { name: 'Qualified', count: 80, color: '#eab308' },
    { name: 'Proposal', count: 40, color: '#f97316' },
    { name: 'Closed', count: 20, color: '#22c55e' },
  ];

  const pipelineStages = stages || defaultStages;

  return (
    <div className="flex flex-col gap-1.5">
      {pipelineStages.map((stage, i) => (
        <div key={i}>
          <div className="flex items-center gap-2.5">
            <div
              className="w-2.5 h-2.5 rounded-full flex-shrink-0"
              style={{ background: stage.color }}
            />
            <div className="flex-1 bg-bg-3 rounded-md px-3 py-2 flex justify-between text-[11px]">
              <span className="text-zinc-300">{stage.name}</span>
              <span className="font-semibold text-white">{stage.count}</span>
            </div>
          </div>
          {i < pipelineStages.length - 1 && (
            <div className="w-0.5 h-3.5 bg-border ml-1.25 my-0.5" style={{ background: '#3f3f46' }} />
          )}
        </div>
      ))}
    </div>
  );
}

export default Pipeline;
