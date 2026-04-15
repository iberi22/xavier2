// Project Card Component for SWAL Operations Dashboard
// Shows CI status, issues, and blockers at a glance

export type ProjectStatus = {
  name: string;
  repo: string;
  tier: 1 | 2 | 3;
  status: "production" | "development" | "maintenance" | "stalled";
  ci_status: "passing" | "failing" | "unknown";
  issues_count: number;
  blockers: string[];
  last_commit: string;
  next_milestone?: string;
};

interface ProjectCardProps {
  project: ProjectStatus;
  onClick?: (project: ProjectStatus) => void;
}

const statusConfig = {
  production: { label: "Production", color: "#22c55e", icon: "OK" },
  development: { label: "Development", color: "#3b82f6", icon: "RUN" },
  maintenance: { label: "Maintenance", color: "#f59e0b", icon: "WARN" },
  stalled: { label: "Stalled", color: "#ef4444", icon: "STOP" },
};

const ciStatusConfig = {
  passing: { label: "CI Passing", icon: "OK" },
  failing: { label: "CI Failing", icon: "ERR" },
  unknown: { label: "CI Unknown", icon: "N/A" },
};

const tierLabels = {
  1: "TIER 1",
  2: "TIER 2",
  3: "TIER 3",
};

const tierColors = {
  1: "#dc2626",
  2: "#2563eb",
  3: "#7c3aed",
};

export function ProjectCard(
  props: ProjectCardProps | { project: string | ProjectStatus },
) {
  const { onClick } = props as ProjectCardProps;
  let project = (props as ProjectCardProps).project;

  if (typeof project === "string") {
    try {
      project = JSON.parse(project) as ProjectStatus;
    } catch {
      return <div>Error parsing project data</div>;
    }
  }

  const status = statusConfig[project.status];
  const ciStatus = ciStatusConfig[project.ci_status];
  const handleActivate = () => onClick?.(project);

  return (
    <div
      className="project-card-shell"
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      style={{
        borderTop: `6px solid ${tierColors[project.tier]}`,
        cursor: onClick ? "pointer" : "default",
      }}
      onClick={handleActivate}
      onKeyDown={(event) => {
        if (!onClick) return;
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          handleActivate();
        }
      }}
    >
      <div className="project-card-header">
        <div className="card-badges">
          <span
            className="cx-badge"
            style={{
              background: `${tierColors[project.tier]}20`,
              color: tierColors[project.tier],
            }}
          >
            {tierLabels[project.tier]}
          </span>
          <span
            className="cx-badge"
            style={{ background: `${status.color}20`, color: status.color }}
          >
            {status.icon} {status.label}
          </span>
        </div>
      </div>

      <h3 className="project-name">{project.name}</h3>
      <p className="project-repo">{project.repo}</p>

      <div className="project-metrics">
        <div className="metric-item">
          <span>{ciStatus.icon}</span>
          <span>{ciStatus.label}</span>
        </div>
        <div className="metric-item">
          <span>ISS</span>
          <span>{project.issues_count} issues</span>
        </div>
      </div>

      {project.blockers.length > 0 ? (
        <div className="blockers-section">
          <strong>Blockers:</strong>
          <ul className="blockers-list">
            {project.blockers.slice(0, 3).map((blocker, idx) => (
              <li key={`${project.repo}-${blocker}-${idx}`}>{blocker}</li>
            ))}
            {project.blockers.length > 3 ? (
              <li className="more-blockers">
                +{project.blockers.length - 3} more
              </li>
            ) : null}
          </ul>
        </div>
      ) : null}

      {project.next_milestone ? (
        <div className="milestone-section">
          <strong>Next:</strong> {project.next_milestone}
        </div>
      ) : null}

      <div className="project-footer">
        <span className="last-commit">
          Last commit: {formatLastCommit(project.last_commit)}
        </span>
      </div>
    </div>
  );
}

function formatLastCommit(dateStr: string): string {
  if (!dateStr) return "Unknown";
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffHours / 24);

    if (diffHours < 1) return "Just now";
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  } catch {
    return dateStr;
  }
}

export default ProjectCard;
