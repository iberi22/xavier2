// Projects Grid Component for SWAL Operations Dashboard
// Shows all projects in a responsive grid with filtering

import { useState, useMemo } from "react";
import { Button, Tag as Badge, Input } from "@openuidev/react-ui";
import { ProjectCard, type ProjectStatus } from "./ProjectCard";
import { ProjectDetail } from "./ProjectDetail";

type FilterStatus = 'all' | 'production' | 'development' | 'maintenance' | 'stalled';
type FilterTier = 'all' | 1 | 2 | 3;

interface ProjectsGridProps {
  projects: ProjectStatus[];
  onRefresh?: () => void;
  lastUpdated?: Date;
}

export function ProjectsGrid({ projects, onRefresh, lastUpdated }: ProjectsGridProps) {
  const [selectedProject, setSelectedProject] = useState<ProjectStatus | null>(null);
  const [statusFilter, setStatusFilter] = useState<FilterStatus>('all');
  const [tierFilter, setTierFilter] = useState<FilterTier>('all');
  const [searchQuery, setSearchQuery] = useState('');

  const filteredProjects = useMemo(() => {
    return projects.filter((project) => {
      // Status filter
      if (statusFilter !== 'all' && project.status !== statusFilter) {
        return false;
      }
      // Tier filter
      if (tierFilter !== 'all' && project.tier !== tierFilter) {
        return false;
      }
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        return (
          project.name.toLowerCase().includes(query) ||
          project.repo.toLowerCase().includes(query) ||
          project.blockers.some(b => b.toLowerCase().includes(query))
        );
      }
      return true;
    });
  }, [projects, statusFilter, tierFilter, searchQuery]);

  const stats = useMemo(() => ({
    total: projects.length,
    production: projects.filter(p => p.status === 'production').length,
    development: projects.filter(p => p.status === 'development').length,
    maintenance: projects.filter(p => p.status === 'maintenance').length,
    stalled: projects.filter(p => p.status === 'stalled').length,
    passingCI: projects.filter(p => p.ci_status === 'passing').length,
    failingCI: projects.filter(p => p.ci_status === 'failing').length,
  }), [projects]);

  // Show detail view if a project is selected
  if (selectedProject) {
    return (
      <ProjectDetail
        project={selectedProject}
        onBack={() => setSelectedProject(null)}
        onRefresh={() => {
          onRefresh?.();
        }}
      />
    );
  }

  return (
    <div className="projects-grid-container">
      <div className="projects-header">
        <div className="projects-title-row">
          <h1>Projects Status Board</h1>
          <Button onClick={onRefresh}>🔄 Refresh All</Button>
        </div>
        
        {lastUpdated && (
          <p className="last-updated">
            Last updated: {lastUpdated.toLocaleTimeString()}
          </p>
        )}

        <div className="stats-row">
          <Badge>Total: {stats.total}</Badge>
          <Badge color="#22c55e">✅ Production: {stats.production}</Badge>
          <Badge color="#3b82f6">🔄 Development: {stats.development}</Badge>
          <Badge color="#f59e0b">⚠️ Maintenance: {stats.maintenance}</Badge>
          <Badge color="#ef4444">❌ Stalled: {stats.stalled}</Badge>
          <span className="stats-divider">|</span>
          <Badge color="#22c55e">✅ CI Passing: {stats.passingCI}</Badge>
          <Badge color="#ef4444">❌ CI Failing: {stats.failingCI}</Badge>
        </div>

        <div className="filters-row">
          <div className="search-box">
            <Input
              placeholder="Search projects..."
              value={searchQuery}
              onChange={(value) => setSearchQuery(value)}
            />
          </div>
          
          <div className="filter-buttons">
            <span className="filter-label">Status:</span>
            {(['all', 'production', 'development', 'maintenance', 'stalled'] as const).map((status) => (
              <Button
                key={status}
                variant={statusFilter === status ? 'primary' : 'secondary'}
                size="small"
                onClick={() => setStatusFilter(status)}
              >
                {status === 'all' ? 'All' : statusConfigIcons[status]} {status === 'all' ? '' : capitalize(status)}
              </Button>
            ))}
          </div>

          <div className="filter-buttons">
            <span className="filter-label">Tier:</span>
            {([1, 2, 3] as const).map((tier) => (
              <Button
                key={tier}
                variant={tierFilter === tier ? 'primary' : 'secondary'}
                size="small"
                onClick={() => setTierFilter(tier === tierFilter ? 'all' : tier)}
              >
                T{tier}
              </Button>
            ))}
            <Button
              variant={tierFilter === 'all' ? 'primary' : 'secondary'}
              size="small"
              onClick={() => setTierFilter('all')}
            >
              All Tiers
            </Button>
          </div>
        </div>
      </div>

      <div className="projects-grid">
        {filteredProjects.length === 0 ? (
          <div className="no-results">
            <p>No projects match your filters.</p>
            <Button variant="secondary" onClick={() => {
              setStatusFilter('all');
              setTierFilter('all');
              setSearchQuery('');
            }}>
              Clear Filters
            </Button>
          </div>
        ) : (
          filteredProjects.map((project) => (
            <ProjectCard
              key={project.repo}
              project={project}
              onClick={setSelectedProject}
            />
          ))
        )}
      </div>

      <div className="projects-footer">
        <span>Showing {filteredProjects.length} of {projects.length} projects</span>
      </div>
    </div>
  );
}

const statusConfigIcons = {
  production: '✅',
  development: '🔄',
  maintenance: '⚠️',
  stalled: '❌',
};

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

export default ProjectsGrid;
