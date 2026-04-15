// Project Detail Component for SWAL Operations Dashboard
// Full view of a single project with all metrics

import { Tag as Badge, Button, Card } from "@openuidev/react-ui";
import type { ProjectStatus } from "./ProjectCard";

interface ProjectDetailProps {
  project: ProjectStatus;
  onBack?: () => void;
  onRefresh?: (project: ProjectStatus) => void;
}

const statusConfig = {
  production: { label: 'Production', color: '#22c55e', icon: '✅' },
  development: { label: 'Development', color: '#3b82f6', icon: '🔄' },
  maintenance: { label: 'Maintenance', color: '#f59e0b', icon: '⚠️' },
  stalled: { label: 'Stalled', color: '#ef4444', icon: '❌' },
};

const ciStatusConfig = {
  passing: { label: 'Passing', color: '#22c55e', icon: '✅' },
  failing: { label: 'Failing', color: '#ef4444', icon: '❌' },
  unknown: { label: 'Unknown', color: '#6b7280', icon: '❓' },
};

const tierLabels = {
  1: 'TIER 1 - CRITICAL',
  2: 'TIER 2 - DEVELOPMENT',
  3: 'TIER 3 - MAINTENANCE',
};

export function ProjectDetail({ project, onBack, onRefresh }: ProjectDetailProps) {
  const status = statusConfig[project.status];
  const ciStatus = ciStatusConfig[project.ci_status];

  return (
    <div className="project-detail">
      <div className="detail-header">
        <Button variant="secondary" onClick={onBack}>
          ← Back to Grid
        </Button>
        <Button onClick={() => onRefresh?.(project)}>
          🔄 Refresh
        </Button>
      </div>

      <Card>
        <div className="detail-title-section">
          <div className="badges-row">
            <Badge color={project.tier === 1 ? '#dc2626' : project.tier === 2 ? '#2563eb' : '#7c3aed'}>
              {tierLabels[project.tier]}
            </Badge>
            <Badge color={status.color}>
              {status.icon} {status.label}
            </Badge>
            <Badge color={ciStatus.color}>
              CI: {ciStatus.icon} {ciStatus.label}
            </Badge>
          </div>
          
          <h1>{project.name}</h1>
          <p className="repo-link">
            <a href={`https://github.com/${project.repo}`} target="_blank" rel="noopener noreferrer">
              {project.repo}
            </a>
          </p>
        </div>

        <div className="detail-metrics-grid">
          <MetricCard
            label="Issues"
            value={String(project.issues_count)}
            icon="📋"
            color={project.issues_count > 20 ? '#ef4444' : project.issues_count > 5 ? '#f59e0b' : '#22c55e'}
          />
          <MetricCard
            label="Blockers"
            value={String(project.blockers.length)}
            icon="🚧"
            color={project.blockers.length > 0 ? '#ef4444' : '#22c55e'}
          />
          <MetricCard
            label="Last Commit"
            value={formatRelativeTime(project.last_commit)}
            icon="📝"
            color="#3b82f6"
          />
        </div>

        {project.blockers.length > 0 && (
          <div className="detail-section">
            <h2>🚧 Blockers</h2>
            <ul className="blockers-detail-list">
              {project.blockers.map((blocker, idx) => (
                <li key={idx} className="blocker-item">
                  {blocker}
                </li>
              ))}
            </ul>
          </div>
        )}

        {project.next_milestone && (
          <div className="detail-section milestone-detail">
            <h2>🎯 Next Milestone</h2>
            <p>{project.next_milestone}</p>
          </div>
        )}

        <div className="detail-section">
          <h2>📊 Project Health</h2>
          <div className="health-bars">
            <HealthBar label="CI Status" status={project.ci_status} />
            <HealthBar label="Issues" status={project.issues_count > 20 ? 'failing' : project.issues_count > 5 ? 'warning' : 'passing'} />
            <HealthBar label="Blockers" status={project.blockers.length > 0 ? 'failing' : 'passing'} />
          </div>
        </div>
      </Card>
    </div>
  );
}

function MetricCard({ label, value, icon, color }: { label: string; value: string; icon: string; color: string }) {
  return (
    <div className="metric-card" style={{ borderLeft: `4px solid ${color}` }}>
      <span className="metric-icon">{icon}</span>
      <div className="metric-content">
        <span className="metric-label">{label}</span>
        <strong className="metric-value">{value}</strong>
      </div>
    </div>
  );
}

function HealthBar({ label, status }: { label: string; status: 'passing' | 'warning' | 'failing' }) {
  const config = {
    passing: { color: '#22c55e', width: '100%', label: 'Healthy' },
    warning: { color: '#f59e0b', width: '60%', label: 'Warning' },
    failing: { color: '#ef4444', width: '30%', label: 'Critical' },
  };
  const bar = config[status];

  return (
    <div className="health-bar-row">
      <span className="health-bar-label">{label}</span>
      <div className="health-bar-track">
        <div 
          className="health-bar-fill" 
          style={{ width: bar.width, backgroundColor: bar.color }}
        />
      </div>
      <span className="health-bar-status" style={{ color: bar.color }}>{bar.label}</span>
    </div>
  );
}

function formatRelativeTime(dateStr: string): string {
  if (!dateStr) return 'Unknown';
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffHours / 24);
    
    if (diffHours < 1) return 'Just now';
    if (diffHours < 24) return `${diffHours} hours ago`;
    if (diffDays < 7) return `${diffDays} days ago`;
    return date.toLocaleDateString();
  } catch {
    return dateStr;
  }
}

export default ProjectDetail;
