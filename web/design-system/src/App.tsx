import React, { useState } from 'react';
import { MetricBig } from './components/MetricBig';
import { StatsRow } from './components/StatsRow';
import { ChartBar } from './components/ChartBar';
import { Progress } from './components/Progress';
import { ListCards } from './components/ListCards';
import { Timeline } from './components/Timeline';
import { Alert } from './components/Alert';
import { HeroBanner } from './components/HeroBanner';
import { Leaderboard } from './components/Leaderboard';
import { Report } from './components/Report';
import { Sparkline } from './components/Sparkline';
import { Comparison } from './components/Comparison';
import { Checklist } from './components/Checklist';
import { IconStat } from './components/IconStat';
import { Pipeline } from './components/Pipeline';
import { Minilist } from './components/Minilist';
import { DividerStats } from './components/DividerStats';
import { BadgeRow } from './components/BadgeRow';

const categories: Record<string, string[]> = {
  Metrics: ['metric', 'stats', 'hero', 'icon', 'divider', 'spark', 'mini'],
  Data: ['chart', 'progress', 'leader', 'pipeline'],
  Lists: ['list', 'timeline', 'check'],
  Docs: ['report', 'comparison'],
  Alerts: ['alert', 'badge'],
};

const templates = [
  { id: 'metric', name: 'MetricBig', desc: 'Hero number with gradient', cat: 'Metrics' },
  { id: 'stats', name: 'StatsRow', desc: '4 KPI boxes', cat: 'Metrics' },
  { id: 'hero', name: 'HeroBanner', desc: 'Gradient hero', cat: 'Metrics' },
  { id: 'icon', name: 'IconStat', desc: 'Icon + big number', cat: 'Metrics' },
  { id: 'divider', name: 'DividerStats', desc: 'Side by side', cat: 'Metrics' },
  { id: 'spark', name: 'Sparkline', desc: 'Mini trend', cat: 'Metrics' },
  { id: 'mini', name: 'Minilist', desc: 'Simple key/value', cat: 'Metrics' },
  { id: 'chart', name: 'ChartBar', desc: 'Simple bars', cat: 'Data' },
  { id: 'progress', name: 'Progress', desc: 'Goals with bars', cat: 'Data' },
  { id: 'leader', name: 'Leaderboard', desc: 'Rankings', cat: 'Data' },
  { id: 'pipeline', name: 'Pipeline', desc: 'Funnel/stages', cat: 'Data' },
  { id: 'list', name: 'ListCards', desc: 'Icon + text cards', cat: 'Lists' },
  { id: 'timeline', name: 'Timeline', desc: 'Events over time', cat: 'Lists' },
  { id: 'check', name: 'Checklist', desc: 'Tasks with status', cat: 'Lists' },
  { id: 'report', name: 'Report', desc: 'Document sections', cat: 'Docs' },
  { id: 'comparison', name: 'Comparison', desc: 'A vs B', cat: 'Docs' },
  { id: 'alert', name: 'Alert', desc: 'Info/Warning/Error', cat: 'Alerts' },
  { id: 'badge', name: 'BadgeRow', desc: 'Tags/badges', cat: 'Alerts' },
];

function renderComponent(id: string) {
  switch (id) {
    case 'metric':
      return <MetricBig label="Recall" value="99.1%" icon="🧠" trend="up" trendValue="2%" />;
    case 'stats':
      return (
        <StatsRow
          stats={[
            { label: 'Memories', value: '415', icon: '💾' },
            { label: 'Recall', value: '99.1%', icon: '🎯' },
            { label: 'Precision', value: '4.45/5', icon: '⚡' },
            { label: 'Latency', value: '56ms', icon: '🚀' },
          ]}
        />
      );
    case 'chart':
      return (
        <ChartBar
          title="Monthly"
          data={[
            { label: 'Jan', value: 40 },
            { label: 'Feb', value: 60 },
            { label: 'Mar', value: 75 },
            { label: 'Apr', value: 90 },
            { label: 'May', value: 85 },
          ]}
        />
      );
    case 'progress':
      return (
        <Progress
          items={[
            { label: 'Development', value: 85, color: '#6366f1' },
            { label: 'Testing', value: 60, color: '#eab308' },
            { label: 'Documentation', value: 40, color: '#22c55e' },
          ]}
        />
      );
    case 'list':
      return (
        <ListCards
          items={[
            { icon: '📊', title: 'Xavier Memory', subtitle: 'AI-powered search' },
            { icon: '🔒', title: 'E2E Encryption', subtitle: 'AES-256-GCM' },
            { icon: '🛡️', title: 'Anticipator', subtitle: 'Prompt injection protection' },
          ]}
        />
      );
    case 'timeline':
      return (
        <Timeline
          items={[
            { date: 'Apr 5, 2026', title: 'Xavier 99.1% recall', description: 'LoCoMo benchmark' },
            { date: 'Apr 4, 2026', title: 'Memory manager', description: 'Decay+consolidate+evict' },
            { date: 'Apr 3, 2026', title: 'SurrealDB v3', description: 'Upgrade complete' },
          ]}
        />
      );
    case 'alert':
      return <Alert type="info" title="Xavier Online" description="Memory system ready at 99.1% recall." />;
    case 'hero':
      return <HeroBanner value="99.1%" label="Recall Achieved" subtitle="Surpassing industry benchmarks" />;
    case 'leader':
      return (
        <Leaderboard
          items={[
            { name: 'ManteniApp', value: '100' },
            { name: 'Xavier', value: '95' },
            { name: 'Synapse', value: '87' },
            { name: 'Edge Hive', value: '72' },
          ]}
        />
      );
    case 'report':
      return <Report />;
    case 'spark':
      return <Sparkline value="+12%" label="Trend" />;
    case 'comparison':
      return (
        <Comparison
          left={{ label: 'Xavier', value: '99.1%' }}
          right={{ label: 'Mem0', value: '66.88%' }}
        />
      );
    case 'check':
      return (
        <Checklist
          items={[
            { label: 'Memory manager implemented', done: true },
            { label: 'E2E encryption designed', done: true },
            { label: 'Cloud deployment', done: false },
            { label: 'Mobile app', done: false },
          ]}
        />
      );
    case 'icon':
      return <IconStat icon="🧠" value="99.1%" label="Recall" color="#6366f1" />;
    case 'pipeline':
      return (
        <Pipeline
          stages={[
            { name: 'Lead', count: 120, color: '#3b82f6' },
            { name: 'Qualified', count: 80, color: '#eab308' },
            { name: 'Proposal', count: 40, color: '#f97316' },
            { name: 'Closed', count: 20, color: '#22c55e' },
          ]}
        />
      );
    case 'mini':
      return (
        <Minilist
          items={[
            { label: 'Memory', value: '415' },
            { label: 'Recall', value: '99.1%' },
            { label: 'Precision', value: '4.45/5' },
            { label: 'Latency', value: '56ms' },
          ]}
        />
      );
    case 'divider':
      return (
        <DividerStats
          left={{ value: '99.1%', label: 'Recall', color: '#6366f1' }}
          right={{ value: '4.45/5', label: 'Precision', color: '#22c55e' }}
        />
      );
    case 'badge':
      return (
        <BadgeRow
          badges={[
            { label: 'Xavier', variant: 'blue' },
            { label: 'Memory', variant: 'green' },
            { label: 'AI', variant: 'purple' },
            { label: 'Secure', variant: 'yellow' },
            { label: 'Fast', variant: 'red' },
          ]}
        />
      );
    default:
      return null;
  }
}

export default function App() {
  const [cat, setCat] = useState('All');

  const filtered =
    cat === 'All' ? templates : templates.filter((t) => categories[cat]?.includes(t.id));

  return (
    <div className="min-h-screen bg-bg text-white" style={{ background: '#09090b' }}>
      {/* Header */}
      <div className="border-b px-6 py-4" style={{ borderColor: '#3f3f46', background: '#18181b' }}>
        <h1 className="text-xl font-bold">🧠 Xavier Design System</h1>
        <p className="text-sm text-zinc-400 mt-1">
          React + TypeScript + TailwindCSS — 18 pre-built infographic components
        </p>
      </div>

      {/* Category Tabs */}
      <div className="flex gap-1 px-6 py-3 border-b overflow-x-auto" style={{ borderColor: '#3f3f46' }}>
        <button
          onClick={() => setCat('All')}
          className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
            cat === 'All' ? 'bg-accent text-white' : 'bg-bg-3 text-zinc-400 hover:text-white'
          }`}
          style={{ background: cat === 'All' ? '#6366f1' : '#27272a' }}
        >
          All
        </button>
        {Object.keys(categories).map((c) => (
          <button
            key={c}
            onClick={() => setCat(c)}
            className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
              cat === c ? 'bg-accent text-white' : 'bg-bg-3 text-zinc-400 hover:text-white'
            }`}
            style={{ background: cat === c ? '#6366f1' : '#27272a' }}
          >
            {c}
          </button>
        ))}
      </div>

      {/* Component Grid */}
      <div className="p-6">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filtered.map((t) => (
            <div key={t.id} className="bg-bg-2 border rounded-xl overflow-hidden" style={{ borderColor: '#3f3f46' }}>
              <div className="px-4 py-2.5 border-b flex items-center gap-2" style={{ borderColor: '#3f3f46' }}>
                <span className="text-xs font-medium text-zinc-300">{t.name}</span>
                <span
                  className="text-[9px] px-1.5 py-0.5 rounded"
                  style={{ background: '#27272a', color: '#71717a' }}
                >
                  {t.desc}
                </span>
              </div>
              <div className="p-4 min-h-36 flex items-center justify-center">
                {renderComponent(t.id)}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
