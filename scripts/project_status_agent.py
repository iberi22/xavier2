#!/usr/bin/env python3
"""
SWAL Project Status Agent
=========================
Reads project status from STATUS.md, fetches real-time data from GitHub API,
and stores project status in Xavier2.

Usage:
    python project_status_agent.py              # Run once
    python project_status_agent.py --poll       # Poll every 5 minutes
"""

import json
import re
import time
import argparse
from datetime import datetime
from pathlib import Path
from typing import Optional

import requests

# Configuration
XAVIER2_URL = "http://localhost:8003"
XAVIER2_TOKEN = "dev-token"
STATUS_FILE = Path("E:/scripts-python/SWAL-Operations-Dashboard/projects/STATUS.md")
GITHUB_API = "https://api.github.com"
GITHUB_HEADERS = {"Accept": "application/vnd.github.v3+json"}


def get_xavier2_headers():
    return {
        "X-Xavier2-Token": XAVIER2_TOKEN,
        "Content-Type": "application/json"
    }


def save_to_xavier2(path: str, content: dict) -> bool:
    """Save project status to Xavier2."""
    try:
        response = requests.post(
            f"{XAVIER2_URL}/memory/add",
            headers=get_xavier2_headers(),
            json={
                "path": path,
                "content": json.dumps(content, indent=2),
                "metadata": {
                    "type": "project_status",
                    "updated_at": datetime.now().isoformat()
                }
            },
            timeout=10
        )
        return response.status_code in (200, 201)
    except Exception as e:
        print(f"  [ERROR] Failed to save to Xavier2: {e}")
        return False


def parse_status_file() -> list[dict]:
    """Parse STATUS.md to extract project information."""
    if not STATUS_FILE.exists():
        print(f"[ERROR] Status file not found: {STATUS_FILE}")
        return []

    content = STATUS_FILE.read_text(encoding='utf-8')
    projects = []

    # Split by project sections (lines starting with ###)
    sections = re.split(r'\n(?=### )', content)

    current_project = None

    for section in sections:
        lines = section.strip().split('\n')

        # Check if this is a project section
        if not lines or not lines[0].startswith('### '):
            continue

        project_name = lines[0].replace('### ', '').strip()

        # Skip header sections
        if project_name in ['Projects Status Board', 'Resumen']:
            continue

        project = {
            'name': project_name,
            'repo': '',
            'tier': 2,  # Default tier
            'status': 'development',
            'ci_status': 'unknown',
            'issues_count': 0,
            'blockers': [],
            'last_commit': '',
            'next_milestone': None,
            'features': []
        }

        # Parse table data from the section
        table_content = '\n'.join(lines[1:])

        # Extract repo
        repo_match = re.search(r'\*\*Repo\*\*.*?\|\s*([^\s]+)', table_content)
        if repo_match:
            repo_path = repo_match.group(1)
            # Convert to iberi22/repo-name format
            if '/' not in repo_path and 'iberi22' not in repo_path:
                repo_name = repo_path.lower().replace(' ', '-')
                project['repo'] = f"iberi22/{repo_name}"
            else:
                project['repo'] = repo_path

        # Extract status
        status_match = re.search(r'\*\*Status\*\*.*?[�🔄⚠️❌✅]\s*([A-Za-z]+)', table_content)
        if status_match:
            status_str = status_match.group(1).lower()
            if status_str in ['production', 'development', 'maintenance', 'stalled']:
                project['status'] = status_str

        # Extract CI status
        ci_match = re.search(r'\*\*CI\*\*.*?[✅⚠️❌]\s*([^\n|]+)', table_content)
        if ci_match:
            ci_str = ci_match.group(1).strip().lower()
            if 'pass' in ci_str or 'ok' in ci_str or '✅' in ci_str:
                project['ci_status'] = 'passing'
            elif 'fail' in ci_str or '❌' in ci_str:
                project['ci_status'] = 'failing'
            elif 'warn' in ci_str or '⚠️' in ci_str:
                project['ci_status'] = 'failing'  # Treat warnings as failing for safety

        # Extract issues count
        issues_match = re.search(r'\*\*Issues\*\*.*?\|\s*(\d+)', table_content)
        if issues_match:
            project['issues_count'] = int(issues_match.group(1))

        # Extract blockers
        blockers_section = re.search(r'\*\*Blockers?\*\*.*?\|\s*(.+?)(?:\n|$)', table_content, re.DOTALL)
        if blockers_section:
            blockers_text = blockers_section.group(1).strip()
            if blockers_text and blockers_text not in ['None', 'None.', '-']:
                # Split by common separators
                blockers = [b.strip() for b in re.split(r'[,;]|\s+-\s+', blockers_text) if b.strip()]
                project['blockers'] = blockers

        # Extract next milestone
        milestone_match = re.search(r'\*\*Next.*?\*\*.*?\|\s*(.+?)(?:\n|$)', table_content, re.DOTALL)
        if milestone_match:
            milestone = milestone_match.group(1).strip()
            if milestone and milestone not in ['-', 'None', '']:
                project['next_milestone'] = milestone

        # Extract tier from heading markers
        if '(iberi22/' in table_content or '**Tier**' in table_content:
            if 'TIER 1' in table_content or project['status'] == 'production':
                project['tier'] = 1

        # Check for last commits
        commits_section = re.search(r'\*\*Last \d+ Commits?:\*\*\s*\n((?:[a-f0-9]+\s+.+\n?)+)', table_content)
        if commits_section:
            first_commit_line = commits_section.group(1).split('\n')[0]
            commit_match = re.search(r'^([a-f0-9]+)', first_commit_line)
            if commit_match:
                project['last_commit'] = commit_match.group(1)

        if project['repo']:
            projects.append(project)

    return projects


def get_github_ci_status(repo: str) -> dict:
    """Get CI status for a repository using GitHub Actions API."""
    try:
        # Get workflow runs
        response = requests.get(
            f"{GITHUB_API}/repos/{repo}/actions/runs",
            headers=GITHUB_HEADERS,
            params={"per_page": 5},
            timeout=10
        )

        if response.status_code == 200:
            data = response.json()
            runs = data.get('workflow_runs', [])

            if runs:
                # Check the latest run status
                latest_run = runs[0]
                status = latest_run.get('status', 'unknown')
                conclusion = latest_run.get('conclusion', 'unknown')

                if conclusion == 'success':
                    return {'ci_status': 'passing', 'last_run': latest_run.get('html_url')}
                elif conclusion in ['failure', 'timed_out']:
                    return {'ci_status': 'failing', 'last_run': latest_run.get('html_url')}
                elif status == 'in_progress':
                    return {'ci_status': 'unknown', 'last_run': latest_run.get('html_url')}
                else:
                    return {'ci_status': 'unknown', 'last_run': latest_run.get('html_url')}
        elif response.status_code == 404:
            return {'ci_status': 'unknown', 'error': 'Repository not found or no Actions'}

        return {'ci_status': 'unknown', 'error': f'Status {response.status_code}'}
    except Exception as e:
        return {'ci_status': 'unknown', 'error': str(e)}


def get_github_issues_count(repo: str) -> int:
    """Get count of open issues for a repository."""
    try:
        response = requests.get(
            f"{GITHUB_API}/repos/{repo}",
            headers=GITHUB_HEADERS,
            timeout=10
        )

        if response.status_code == 200:
            data = response.json()
            return data.get('open_issues_count', 0)

        return 0
    except Exception as e:
        print(f"  [ERROR] Failed to get issues for {repo}: {e}")
        return 0


def get_github_last_commit(repo: str) -> Optional[str]:
    """Get the date of the last commit."""
    try:
        response = requests.get(
            f"{GITHUB_API}/repos/{repo}/commits",
            headers=GITHUB_HEADERS,
            params={"per_page": 1},
            timeout=10
        )

        if response.status_code == 200:
            data = response.json()
            if data and len(data) > 0:
                return data[0].get('commit', {}).get('committer', {}).get('date', '')

        return None
    except Exception as e:
        print(f"  [ERROR] Failed to get last commit for {repo}: {e}")
        return None


def enrich_project_data(project: dict) -> dict:
    """Enrich project data with real-time GitHub information."""
    repo = project.get('repo', '')
    if not repo or repo == 'iberi22/':
        return project

    print(f"  Fetching data for {repo}...")

    # Get CI status
    ci_data = get_github_ci_status(repo)
    project['ci_status'] = ci_data.get('ci_status', project.get('ci_status', 'unknown'))

    # Get issues count (only if not already set from STATUS.md)
    if project.get('issues_count', 0) == 0:
        project['issues_count'] = get_github_issues_count(repo)

    # Get last commit
    last_commit = get_github_last_commit(repo)
    if last_commit:
        project['last_commit'] = last_commit

    # Add metadata
    project['github_url'] = f"https://github.com/{repo}"
    project['last_updated'] = datetime.now().isoformat()

    return project


def save_projects_overview(projects: list[dict]) -> bool:
    """Save overview of all projects to Xavier2."""
    overview = {
        'updated_at': datetime.now().isoformat(),
        'total_projects': len(projects),
        'by_status': {},
        'by_tier': {},
        'ci_health': {},
        'projects': [p['name'] for p in projects]
    }

    # Aggregate stats
    for project in projects:
        status = project.get('status', 'unknown')
        tier = project.get('tier', 2)
        ci = project.get('ci_status', 'unknown')

        overview['by_status'][status] = overview['by_status'].get(status, 0) + 1
        overview['by_tier'][f"tier_{tier}"] = overview['by_tier'].get(f"tier_{tier}", 0) + 1

        if ci == 'passing':
            overview['ci_health']['passing'] = overview['ci_health'].get('passing', 0) + 1
        elif ci == 'failing':
            overview['ci_health']['failing'] = overview['ci_health'].get('failing', 0) + 1
        else:
            overview['ci_health']['unknown'] = overview['ci_health'].get('unknown', 0) + 1

    return save_to_xavier2("sweat-operations/projects/overview", overview)


def run_status_update():
    """Main function to run one status update cycle."""
    print(f"\n[{datetime.now().isoformat()}] Starting Project Status Update")
    print("=" * 50)

    # Parse projects from STATUS.md
    print("\n[1/3] Parsing STATUS.md...")
    projects = parse_status_file()
    print(f"  Found {len(projects)} projects")

    # Enrich with GitHub data
    print("\n[2/3] Fetching GitHub data...")
    for i, project in enumerate(projects):
        print(f"  [{i+1}/{len(projects)}] Processing {project['name']} ({project['repo']})...")
        projects[i] = enrich_project_data(project)
        time.sleep(0.3)  # Rate limiting

    # Save individual project status to Xavier2
    print("\n[3/3] Saving to Xavier2...")
    saved_count = 0
    for project in projects:
        path = f"sweat-operations/projects/{project['name'].lower().replace(' ', '-')}/status"
        if save_to_xavier2(path, project):
            saved_count += 1

    # Save overview
    if save_projects_overview(projects):
        saved_count += 1

    print(f"\n[SUMMARY] Saved {saved_count}/{len(projects) + 1} entries to Xavier2")
    print(f"Completed at: {datetime.now().isoformat()}")

    return projects


def main():
    parser = argparse.ArgumentParser(description='SWAL Project Status Agent')
    parser.add_argument('--poll', action='store_true', help='Poll every 5 minutes')
    parser.add_argument('--interval', type=int, default=300, help='Poll interval in seconds (default: 300)')
    args = parser.parse_args()

    if args.poll:
        print(f"Starting Project Status Agent in polling mode (interval: {args.interval}s)")
        while True:
            run_status_update()
            print(f"\nSleeping for {args.interval} seconds until next update...")
            time.sleep(args.interval)
    else:
        run_status_update()


if __name__ == "__main__":
    main()
