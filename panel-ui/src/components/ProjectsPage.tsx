// Projects Page Component for SWAL Operations Dashboard
// Full page view with projects grid and real-time updates

import { useState, useEffect, useCallback } from "react";
import { ProjectsGrid, type ProjectStatus } from "./ProjectsGrid";

// Project status from API
async function fetchProjectStatus(token: string): Promise<ProjectStatus[]> {
  try {
    const response = await fetch("/panel/api/projects/status", {
      headers: { "X-Xavier-Token": token },
    });
    if (!response.ok) {
      throw new Error("Failed to fetch project status");
    }
    return response.json() as Promise<ProjectStatus[]>;
  } catch {
    // Return mock data if API not available
    return getMockProjects();
  }
}

function getMockProjects(): ProjectStatus[] {
  return [
    {
      name: "Synapse Protocol",
      repo: "iberi22/synapse-protocol",
      tier: 1,
      status: "development",
      ci_status: "failing",
      issues_count: 25,
      blockers: ["SurrealDB 3.x migration", "P2P Node Discovery blocked"],
      last_commit: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
      next_milestone: "SurrealDB migration complete"
    },
    {
      name: "ManteniApp",
      repo: "iberi22/manteniapp",
      tier: 1,
      status: "production",
      ci_status: "passing",
      issues_count: 10,
      blockers: [],
      last_commit: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
      next_milestone: "Close first enterprise client"
    },
    {
      name: "Xavier",
      repo: "iberi22/xavier",
      tier: 2,
      status: "development",
      ci_status: "failing",
      issues_count: 14,
      blockers: ["Documentation", "Marketing"],
      last_commit: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
    },
    {
      name: "Gestalt-Rust",
      repo: "iberi22/gestalt-rust",
      tier: 2,
      status: "development",
      ci_status: "passing",
      issues_count: 10,
      blockers: [],
      last_commit: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    },
    {
      name: "WorldExams",
      repo: "iberi22/worldexams",
      tier: 3,
      status: "maintenance",
      ci_status: "passing",
      issues_count: 54,
      blockers: [],
      last_commit: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    },
    {
      name: "Moonshot Trading Bot",
      repo: "iberi22/moonshot-trading-bot",
      tier: 2,
      status: "development",
      ci_status: "failing",
      issues_count: 4,
      blockers: ["Strategy validation"],
      last_commit: new Date(Date.now() - 12 * 60 * 60 * 1000).toISOString(),
    },
  ];
}

interface ProjectsPageProps {
  token: string;
  onBack?: () => void;
}

export function ProjectsPage({ token, onBack }: ProjectsPageProps) {
  const [projects, setProjects] = useState<ProjectStatus[]>([]);
  const [lastUpdated, setLastUpdated] = useState<Date | undefined>();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadProjects = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const data = await fetchProjectStatus(token);
      setProjects(data);
      setLastUpdated(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load projects");
    } finally {
      setIsLoading(false);
    }
  }, [token]);

  // Initial load
  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  // Poll every 5 minutes
  useEffect(() => {
    const interval = setInterval(() => {
      void loadProjects();
    }, 5 * 60 * 1000);

    return () => clearInterval(interval);
  }, [loadProjects]);

  if (isLoading && projects.length === 0) {
    return (
      <div className="projects-loading">
        <div className="loading-spinner" />
        <p>Loading projects...</p>
      </div>
    );
  }

  if (error && projects.length === 0) {
    return (
      <div className="projects-error">
        <p>Error: {error}</p>
        <button type="button" onClick={() => void loadProjects()}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="projects-page">
      <div className="projects-nav">
        {onBack && (
          <button type="button" className="back-button" onClick={onBack}>
            ← Back
          </button>
        )}
        <span className="projects-badge">📊 Projects Status</span>
        {isLoading && <span className="refreshing">Refreshing...</span>}
      </div>

      <ProjectsGrid
        projects={projects}
        onRefresh={() => void loadProjects()}
        lastUpdated={lastUpdated}
      />
    </div>
  );
}

export default ProjectsPage;
