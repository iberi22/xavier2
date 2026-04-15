---
title: "Protocol Propagation System"
type: DOCUMENTATION
id: "doc-protocol-propagation"
created: 2025-12-06
updated: 2025-12-06
agent: copilot
model: claude-sonnet-4
requested_by: user
summary: |
  Automated system to propagate Git-Core Protocol updates to all managed repositories.
  Creates PRs or Issues automatically when new versions are released.
keywords: [propagation, versioning, automation, deployment]
tags: ["#automation", "#versioning", "#deployment"]
project: Git-Core-Protocol
---

# 🚀 Protocol Propagation System

> **"Update once, propagate everywhere"**

## Overview

The Protocol Propagation System automatically distributes Git-Core Protocol updates to all your repositories when you create a new version.

---

## 📋 Quick Start

### Creating a New Version

```powershell
# Automatic version bump (patch: 3.0.0 → 3.0.1)
.\scripts\release-protocol.ps1

# Specific version
.\scripts\release-protocol.ps1 -Version "3.1.0"

# Major version bump (3.0.0 → 4.0.0)
.\scripts\release-protocol.ps1 -Type major

# Minor version bump (3.0.0 → 3.1.0)
.\scripts\release-protocol.ps1 -Type minor

# Dry run (see what would happen)
.\scripts\release-protocol.ps1 -DryRun
```

### What Happens Automatically

1. **Version file updated** (`VERSION`)
2. **Git tag created** (`v3.1.0`)
3. **Tag pushed** to GitHub
4. **Workflow triggered** (`protocol-propagation.yml`)
5. **PRs created** in all target repos

---

## 🎯 Update Types

| Type | What's Updated |
|------|----------------|
| `workflows` | `.github/workflows/*` |
| `agents` | `AGENTS.md`, `.cursorrules`, `.github/copilot-instructions.md` |
| `scripts` | `scripts/*` |
| `full` | Everything above |

### Specifying Update Type

```powershell
# Only update workflows
.\scripts\release-protocol.ps1 -UpdateType workflows

# Full protocol update
.\scripts\release-protocol.ps1 -UpdateType full
```

---

## 🔧 Configuration

### `.propagation.json`

Configure which repos receive updates:

```json
{
  "target_repos": [
    {
      "name": "iberi22/software-factory",
      "enabled": true,
      "update_types": ["workflows", "agents", "scripts"],
      "auto_merge": false,
      "priority": 1
    }
  ]
}
```

**Fields:**

- `enabled`: Enable/disable propagation for this repo
- `update_types`: Which types of updates to apply
- `auto_merge`: Auto-merge PRs (requires CI passing)
- `priority`: Order of execution (1 = highest)

### Excluding Repos

Set `"enabled": false` in `.propagation.json`:

```json
{
  "name": "iberi22/archived-repo",
  "enabled": false
}
```

---

## 📊 Workflow Execution

### Automatic Trigger

When you push a tag:

```bash
git tag v3.1.0
git push origin v3.1.0
```

### Manual Trigger

Via GitHub Actions UI or CLI:

```bash
gh workflow run protocol-propagation.yml \
  --field update_type=workflows \
  --field create_pr=true
```

**Parameters:**

- `target_repos`: Comma-separated list (empty = all)
- `update_type`: workflows | agents | scripts | full
- `create_pr`: true (PR) or false (Issue)

---

## 🔍 Monitoring

### Check Workflow Status

```bash
gh run list --workflow protocol-propagation.yml --limit 5
```

### View Created PRs

```bash
# In a target repo
gh pr list --label "protocol-update"
```

### Summary

After workflow completes, check the summary:

```
https://github.com/iberi22/Git-Core-Protocol/actions
```

---

## 📝 PR/Issue Content

### Pull Request

**Title:** `🚀 Git-Core Protocol v3.1.0 Update`

**Body:**

- Version information
- Update type
- Changelog link
- Testing checklist

**Labels:** `dependencies`, `protocol-update`

### Issue (Alternative)

**Title:** `📦 Git-Core Protocol v3.1.0 Available`

**Body:**

- Notification of new version
- Branch name with updates
- Instructions to review and merge

---

## 🛠️ Troubleshooting

### PR Already Exists

If a PR for the same version exists, the workflow skips creating a new one.

**Solution:** Close the old PR or merge it first.

### Repo Not Found

Some repos may not be accessible to the GitHub token.

**Solution:** Ensure `GITHUB_TOKEN` has access to all target repos.

### No Changes Detected

If the target repo already has the latest version, no PR is created.

**Solution:** This is expected behavior.

### Merge Conflicts

If the target repo has diverged significantly, merge conflicts may occur.

**Solution:** Manually resolve conflicts in the created PR.

---

## 🎯 Best Practices

### 1. **Semantic Versioning**

Follow semver:

- `major`: Breaking changes
- `minor`: New features, backwards compatible
- `patch`: Bug fixes

### 2. **Test Before Release**

```powershell
# Test in dry run mode first
.\scripts\release-protocol.ps1 -DryRun
```

### 3. **Update Changelog**

Document changes in `CHANGELOG.md` before creating a version.

### 4. **Review PRs**

Don't auto-merge. Review each PR to ensure compatibility.

### 5. **Staged Rollout**

Use priorities in `.propagation.json`:

- Priority 1: Critical repos (test first)
- Priority 2: Standard repos
- Priority 3: Low-priority repos

Merge Priority 1 PRs, verify, then proceed to Priority 2.

---

## 📦 Version Management

### Current Version

```bash
cat VERSION
```

### Version History

```bash
git tag --list | Sort-Object -Descending
```

### Rollback

If a version causes issues:

```bash
# Create a new version with fixes
.\scripts\release-protocol.ps1 -Version "3.1.1" -Message "fix: rollback breaking changes"
```

---

## 🔐 Security Considerations

### GitHub Token Permissions

The workflow requires:

- `contents: read` on Protocol repo
- `contents: write` on target repos (for creating branches)
- `pull-requests: write` on target repos (for creating PRs)

### Sensitive Files

Excluded by default (see `.propagation.json`):

- `.env`
- `credentials.json`
- `token.json`
- `secrets/*`

---

## 📊 Metrics

Track propagation success:

```bash
# Count PRs created
gh pr list --label "protocol-update" --search "created:>2025-12-01"

# Count merged updates
gh pr list --label "protocol-update" --state merged
```

---

## 🚀 Example Workflow

### Scenario: Add Self-Healing Workflow

1. **Develop feature** in Protocol repo
2. **Test locally**
3. **Update VERSION**:

   ```powershell
   .\scripts\release-protocol.ps1 -Type minor
   ```

4. **Monitor propagation**:

   ```bash
   gh run watch
   ```

5. **Review PRs** in target repos
6. **Merge PRs** after testing
7. **Verify** self-healing works in each repo

---

## 📖 Related Documentation

- [Semantic Versioning](https://semver.org/)
- [GitHub Actions workflow_dispatch](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#workflow_dispatch)
- [Git Tagging](https://git-scm.com/book/en/v2/Git-Basics-Tagging)

---

## 🆘 Support

Issues with propagation? Check:

1. Workflow logs: `https://github.com/iberi22/Git-Core-Protocol/actions`
2. Target repo status: `gh repo view <repo>`
3. PR creation errors: Check job logs in workflow

---

*Last updated: December 2025*
*Protocol Version: 3.0.0*
