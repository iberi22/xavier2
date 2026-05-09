import React from 'react';

export interface ReportSection {
  title: string;
  content: string;
}

export interface ReportProps {
  title?: string;
  date?: string;
  sections?: ReportSection[];
}

export function Report({ title = 'Quarterly Report', date = 'Q1 2026', sections }: ReportProps) {
  const defaultSections: ReportSection[] = [
    { title: 'Summary', content: 'Major improvements across all metrics.' },
    { title: 'Key Achievements', content: 'Launched Xavier with 99.1% recall.' },
    { title: 'Next Steps', content: 'Focus on enterprise features.' },
  ];

  const reportSections = sections || defaultSections;

  return (
    <div className="bg-bg-3 rounded-lg p-4">
      <div className="mb-3 pb-2.5 border-b" style={{ borderColor: '#3f3f46' }}>
        <div className="text-sm font-bold text-white">{title}</div>
        <div className="text-[10px] text-zinc-500 mt-0.5">{date}</div>
      </div>
      {reportSections.map((section, i) => (
        <div key={i} className="mb-2.5 last:mb-0">
          <div className="text-[10px] font-semibold uppercase text-zinc-400 mb-1">
            {section.title}
          </div>
          <p className="text-xs text-zinc-400 leading-relaxed">{section.content}</p>
        </div>
      ))}
    </div>
  );
}

export default Report;
