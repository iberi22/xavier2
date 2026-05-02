# Multi-Agent Development Pipeline

_Avoid the traps we learned the hard way._

---

## The Golden Rule

**Every agent that finishes must be verified BEFORE moving to the next one.**

Silent failure (exit 0, no changes) is the most expensive error because it looks like success.

---

## Pre-Flight Checklist

Before launching ANY agent:

### 1. Verify clean state
```bash
# In the worktree we're about to use
git status --short
# Should show nothing uncommitted
```

### 2. Ensure master is up-to-date
```bash
git fetch origin master
git log --oneline origin/master -3
# If behind, update before creating new branches
```

### 3. Create isolated worktree
```bash
git worktree add E:\temp\issue-XXX -b fix/issue-XXX master
```
**Never run parallel agents in the same worktree or repo.**

### 4. For each issue, one worktree
Each issue = one worktree = one agent = one PR.

---

## Agent Launch Protocol

### Step 1 — Construct the prompt

A good prompt includes:
- **Issue number and title** — so agent knows what it's fixing
- **Specific file(s) to modify** — no wandering
- **Acceptance criteria** — what "done" looks like
- **Fetch + merge instruction** — prevents stale conflicts
- **Verification instruction** — how to confirm the fix works

Example template:
```
Fix issue #137: qmd_memory.rs is 105KB - modularize.

1. Read src/app/qmd_memory.rs
2. Identify logical sections (search, add, index, cache, etc.)
3. Extract each section into src/app/qmd_memory/*.rs modules
4. Keep src/app/qmd_memory.rs as re-export hub
5. Update lib.rs to include new modules
6. Run cargo check --lib to verify compilation
7. Commit with message "refactor: split qmd_memory.rs into modules (closes #137)"
```

### Step 2 — Launch with right tool

| Tool | When to use | Fallback |
|------|-------------|----------|
| `codex.ps1 exec --full-auto` | General coding, Rust preferred | `codex.ps1 exec --yolo` |
| `codex.ps1 exec --yolo` | Fast, trusted code | `codex.ps1 exec --full-auto` |
| `opencode.ps1 run` | Simple tasks with MiniMax | `codex.ps1 exec --yolo` |

**On Windows: prefer Codex over OpenCode** — OpenCode sandbox fails on Windows paths (`valid workspace SID` error).

### Step 3 — Launch in background with PTY

```bash
exec command:"codex.ps1 exec --full-auto 'YOUR PROMPT HERE'" pty:true workdir:E:\temp\issue-XXX background:true
```

### Step 4 — Monitor with process:log

Not `poll` — check `log` to see what's actually happening:

```bash
process action:log sessionId:XXX limit:30
```

`poll` only tells you if it's still running. `log` shows real progress.

---

## Post-Completion Verification (MANDATORY)

After EVERY agent completes:

```bash
# 1. Check what changed
cd E:\temp\issue-XXX
git diff HEAD --stat
# If empty → agent did nothing, respawn immediately

# 2. Verify clean status
git status --short
# Should show only the files we intend to commit

# 3. Push
git push -u origin fix/issue-XXX
```

If git diff is empty:
1. Agent silently succeeded (code was already correct) → close PR, move on
2. Agent couldn't find files → respawn with more specific paths
3. Agent hit sandbox error → switch to Codex

---

## Error Recovery Playbook

### Problem: Agent exits 0 but no changes

**Cause:** Agent couldn't find relevant code, or decided existing code was "already correct."

**Fix:**
```bash
# Immediately check
git diff HEAD
# If empty:
# - Check what files exist in target area
# - Respawn with explicit file paths in prompt
```

### Problem: OpenCode "valid workspace SID" error on Windows

**Cause:** OpenCode sandbox doesn't work with certain Windows paths.

**Fix:** Switch to Codex:
```bash
codex.ps1 exec --full-auto "YOUR TASK"
```

### Problem: OpenCode ProviderModelNotFoundError

**Cause:** Wrong model name format.

**Fix:** Use exact `MiniMax-M2.7`, not `minimax/minimax-m2.7`.

### Problem: PR has merge conflicts

**Cause:** Agent pushed without merging master first.

**Prevention:** Always add this to agent prompt:
```
Before pushing, run: git fetch origin master && git merge origin/master
If conflicts, use: git checkout --ours <file>
Then commit and push.
```

**Fix:**
```bash
cd E:\temp\issue-XXX
git fetch origin master
git merge origin/master
# If conflict: git checkout --ours <conflicting-file>
git add <files>
git commit -m "Merge master"
git push
```

### Problem: Multiple agents overwrite same files

**Cause:** Two worktrees modified same file without coordination.

**Prevention:** One issue per worktree. Never modify same file in two worktrees.

**Fix:** Close duplicate PRs, keep one, manually reconcile if needed.

---

## Parallel Execution Rules

### DO ✓
- Run multiple Codex agents in separate worktrees (one per issue)
- Run OpenCode in separate worktrees (one per issue)
- Monitor all with `process action:list`

### DON'T ✗
- Run OpenCode and Codex on same file at same time
- Run two agents in same worktree
- Run agents without worktrees on main repo

---

## PR Creation and Merge Protocol

### 1. After agent pushes, verify push succeeded
```bash
# Should see: branch set up to track origin/fix/issue-XXX
```

### 2. Create PR
```bash
cd E:\temp\issue-XXX
gh pr create --title "fix: issue title (closes #NNN)" \
  --body "## Summary\n\n## Changes\n\n## Testing" \
  --base master --head fix/issue-XXX
```

### 3. Immediate merge check
```bash
gh pr merge --squash NNN
# If fails: check conflict files with gh pr view NNN --json files
```

### 4. If conflict:
- Close the conflicting PR
- Create new clean branch from latest master
- Re-launch agent with merge instruction

---

## Quick Reference

### Launch an agent (correct pattern)
```bash
exec command:"codex.ps1 exec --full-auto 'Fix issue #84. Check gh issue view 84. Edit files, commit with message \"fix: close #84\" and push.'" pty:true workdir:E:\temp\issue-084 background:true
```

### Check progress
```bash
process action:log sessionId:XXX limit:30
```

### Check result
```bash
cd E:\temp\issue-084; git diff HEAD --stat; git status --short
```

### Force push after merge fix
```bash
git push --force-with-lease
```

---

## Common Issues → Quick Fixes

| Issue | Fix |
|-------|-----|
| `codex exec` hangs | Use `pty:true` |
| No changes after agent | Respawn with explicit file paths |
| Merge conflict | Fetch + merge + `git checkout --ours` |
| OpenCode fails on Windows | Use `codex.ps1 exec` instead |
| Model not found | Use `MiniMax-M2.7` (exact) |
| Agent exits 0 silently | Always check `git diff HEAD` |
| Worktree lock | `git worktree remove E:\temp\XXX --force` |
| Stale branch after merge | Fetch + reset: `git fetch origin; git reset --hard origin/master` |

---

_Revised 2026-05-02 after sprint lessons._
