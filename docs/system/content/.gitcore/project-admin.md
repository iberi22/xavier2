# Project Administration - GitCore Protocol

## Overview

GitCore Protocol includes project administration features for managing software projects, teams, tasks, and releases across an organization.

---

## Features

### 1. Project Registry

**Purpose:** Central registry of all software projects.

**Schema:**

```yaml
project:
  id: string              # Unique identifier
  name: string            # Project name
  description: string     # Project description
  repository: string      # Git repository URL
  language: string        # Primary language
  framework: string       # Framework used
  team_id: string         # Associated team
  status: enum            # active, archived, deprecated
  created_at: timestamp
  updated_at: timestamp
  tags: string[]          # Filtering tags
```

**Commands:**

```bash
# Register new project
gitcore project add --name "ManteniApp" --repo "iberi22/manteniapp" --lang "TypeScript"

# List projects
gitcore project list --status active

# Get project details
gitcore project show manteniapp
```

---

### 2. Task Tracking

**Purpose:** Track tasks, issues, and sprints.

**Schema:**

```yaml
task:
  id: string              # Unique identifier
  project_id: string      # Associated project
  title: string           # Task title
  description: string      # Detailed description
  type: enum              # feature, bug, chore, release
  status: enum            # todo, in_progress, review, done
  priority: enum          # low, medium, high, critical
  assignee: string        # Assigned team member
  reporter: string        # Task creator
  sprint: string          # Sprint name
  labels: string[]        # Task labels
  due_date: timestamp
  created_at: timestamp
  updated_at: timestamp
```

**Commands:**

```bash
# Create task
gitcore task create --project manteniapp --title "Fix login bug" --priority high

# List tasks
gitcore task list --project manteniapp --status todo

# Move task to in progress
gitcore task start TASK-123

# Complete task
gitcore task done TASK-123
```

---

### 3. Team Management

**Purpose:** Manage team members and roles.

**Schema:**

```yaml
team:
  id: string              # Unique identifier
  name: string            # Team name
  description: string     # Team purpose
  members: Member[]       # Team members
  projects: string[]      # Associated projects

member:
  user_id: string
  role: enum              # admin, lead, developer, tester
  joined_at: timestamp

role_permissions:
  admin: [create, read, update, delete, manage_team]
  lead: [create, read, update, manage_project]
  developer: [create, read, update]
  tester: [read, test, report]
```

**Commands:**

```bash
# Create team
gitcore team create --name "Backend" --description "Backend developers"

# Add member
gitcore team add-member --team backend --user belal --role lead

# List team members
gitcore team list --name backend
```

---

### 4. Release Management

**Purpose:** Manage version releases and changelogs.

**Schema:**

```yaml
release:
  id: string              # Unique identifier
  project_id: string
  version: string         # Semantic version (1.0.0)
  status: enum            # planned, in_progress, released
  changelog: string       # Release notes
  artifacts: Artifact[]   # Build artifacts
  deployed_at: timestamp

artifact:
  name: string
  url: string
  platform: string        # linux, windows, docker
  architecture: string    # x64, arm64
```

**Commands:**

```bash
# Create release
gitcore release create --project manteniapp --version "1.0.0"

# Add changelog entry
gitcore release changelog --project manteniapp --version "1.0.0" --add "Added user authentication"

# Deploy release
gitcore release deploy --project manteniapp --version "1.0.0"
```

---

### 5. Dependency Tracker

**Purpose:** Track project dependencies and security updates.

**Schema:**

```yaml
dependency:
  name: string            # Package name
  version: string         # Current version
  latest: string          # Latest available
  type: enum              # production, development
  project_id: string
  update_available: boolean
  security_issues: Issue[]

issue:
  id: string
  severity: enum          # low, medium, high, critical
  description: string
  cve: string            # CVE identifier if applicable
```

**Commands:**

```bash
# Check dependencies
gitcore deps check --project manteniapp

# Update dependency
gitcore deps update --project manteniapp --package react

# Security audit
gitcore deps audit --project manteniapp
```

---

### 6. Cost Analytics

**Purpose:** Track project costs and resource usage.

**Schema:**

```yaml
cost_metrics:
  project_id: string
  period: string          # monthly, quarterly
  compute_cost: number    # Cloud compute costs
  storage_cost: number    # Storage costs
  license_cost: number   # Software licenses
  total_cost: number
  currency: string        # USD, CLP, etc.

resource_usage:
  cpu_hours: number
  storage_gb: number
  bandwidth_gb: number
  api_calls: number
```

**Commands:**

```bash
# Get cost report
gitcore costs report --project manteniapp --period monthly

# Compare projects
gitcore costs compare --period quarterly
```

---

## Integration Points

### GitHub Integration

```yaml
integration:
  github:
    webhooks:
      - pull_request
      - push
      - issues
    automation:
      auto_assign_tasks: true
      auto_label_releases: true
```

### Agent Integration

```yaml
agent_workflow:
  on_pr_open:
    - analyze_code
    - run_tests
    - update_task_status

  on_task_done:
    - generate_changelog
    - check_deps
    - notify_team
```

---

## File Locations

| Feature | Storage Location |
|---------|------------------|
| Projects | `.gitcore/projects/` |
| Tasks | `.gitcore/tasks/` |
| Teams | `.gitcore/teams/` |
| Releases | `.gitcore/releases/` |
| Dependencies | `.gitcore/deps/` |
| Costs | `.gitcore/costs/` |

---

*Document version: 1.0*
*Last updated: 2026-03-15*
