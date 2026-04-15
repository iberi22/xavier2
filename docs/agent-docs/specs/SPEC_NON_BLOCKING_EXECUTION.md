---
title: "Non-Blocking Execution Protocol"
type: PROTOCOL_IMPROVEMENT
id: "protocol-non-blocking"
created: 2025-12-07
updated: 2025-12-07
agent: copilot
model: claude-sonnet-4
requested_by: user
summary: |
  Protocol improvement to prevent blocking chat when executing long-running commands.
  Uses background execution and result files instead of showing output in chat.
keywords: [protocol, optimization, background-execution, non-blocking]
tags: ["#protocol", "#improvement", "#execution"]
project: Git-Core-Protocol
priority: high
status: approved
---

# 🚀 Non-Blocking Execution Protocol

## 🎯 Problem

AI agents can block the chat conversation when executing long-running commands that produce large outputs:

❌ **Blocking behaviors:**
- Showing full test output in chat (25 tests × verbose output)
- Displaying git logs, diffs, or large file contents
- Running commands that take >5 seconds
- Streaming command output to chat

**User Experience Impact:**
- User cannot send new messages while command runs
- Chat becomes unresponsive
- Large outputs clutter the conversation
- Difficult to follow the actual task progress

---

## ✅ Solution: Background + Results Files

### Principles

1. **Execute in background** - Use `-isBackground: true` for long commands
2. **Write to files** - Store results in temp files, not chat
3. **Summarize only** - Show 1-3 line summary in chat
4. **Reference files** - Let user check details if needed

---

## 📋 Implementation Rules

### Rule 1: Detect Long-Running Commands

**Always use background execution for:**

| Command Type | Examples | Threshold |
|--------------|----------|-----------|
| Tests | `npm test`, `pytest`, `cargo test` | Any test suite |
| Builds | `npm build`, `cargo build`, `docker build` | Any build command |
| Git operations | `git log`, `git diff`, large `git status` | >10 lines expected |
| CI simulations | Running workflows, validation scripts | Always |
| File processing | Parsing large files, batch operations | >5 files |

### Rule 2: Use Result Files

```powershell
# ❌ WRONG - Blocks chat
$output = npm test
Write-Host $output

# ✅ RIGHT - Background + file
npm test > test-results.txt 2>&1 &
Write-Host "✅ Tests running in background. Results: test-results.txt"
```

### Rule 3: Concise Summaries

**Show in chat:**
```
✅ Tests complete: 25/25 passed
📄 Details: test-results.txt
```

**DON'T show in chat:**
```
Running test 1...
  ✓ should detect public repo
  ✓ should detect private repo
  ✓ should set aggressive mode
[... 200 more lines ...]
```

---

## 🛠️ Practical Patterns

### Pattern 1: Test Execution

```powershell
# Execute in background
$testJob = Start-Job -ScriptBlock {
    ./scripts/test-adaptive-system.ps1
}

# Wait silently
Wait-Job $testJob | Out-Null

# Get results
$result = Receive-Job $testJob
$result | Out-File -FilePath "test-results.txt"

# Summarize
$passed = ($result | Select-String "PASSED").Count
$failed = ($result | Select-String "FAILED").Count

Write-Host "✅ Tests: $passed passed, $failed failed"
Write-Host "📄 Full results: test-results.txt"
```

### Pattern 2: Git Operations

```powershell
# ❌ WRONG
git log --oneline -100

# ✅ RIGHT
git log --oneline -100 > git-history.txt
$commits = (Get-Content git-history.txt).Count
Write-Host "📊 $commits commits in history. See: git-history.txt"
```

### Pattern 3: Build Operations

```powershell
# ❌ WRONG
npm run build

# ✅ RIGHT
$buildJob = Start-Job { npm run build 2>&1 }
Wait-Job $buildJob | Out-Null
Receive-Job $buildJob | Out-File build-log.txt

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Build successful"
} else {
    Write-Host "❌ Build failed. See: build-log.txt"
}
```

---

## 📁 Result Files Location

**Use temporary directory:**
```powershell
$tempDir = Join-Path $env:TEMP "git-core-results"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

$resultFile = Join-Path $tempDir "test-results-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
```

**Or project temp:**
```
.git-core/
  results/
    test-results-20251207-143022.txt
    build-log-20251207-143125.txt
    git-history-20251207-143200.txt
```

---

## 🎯 Integration with Git-Core Protocol

### Update to copilot-instructions.md

Add section:

```markdown
## 🚀 Execution Guidelines

### Non-Blocking Execution

**ALWAYS use background execution for:**
- Test suites (npm test, pytest, cargo test)
- Build commands (npm build, cargo build)
- Git operations returning >10 lines
- CI workflow simulations
- Batch file processing

**Pattern:**
1. Execute in background: `command > result.txt 2>&1 &`
2. Show 1-line status: "✅ Running..."
3. Summarize result: "✅ Done: 25/25 passed"
4. Reference file: "📄 Details: result.txt"

**NEVER:**
- Stream long output to chat
- Block conversation with running commands
- Display full test results (>20 lines)
```

### Update to AGENTS.md

Add to "Communication" section:

```markdown
### Long-Running Operations

For commands that take >5 seconds or produce >20 lines:

1. **Execute in background**
2. **Write results to temp file**
3. **Show concise summary only**

Example:
```
✅ Tests complete: 25/25 passed (2.3s)
📄 Full output: .git-core/results/test-20251207.txt
```

---

## 📊 Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| Chat blocking | 30-60s per test run | 0s (background) |
| Output clutter | 200+ lines | 2-3 lines |
| User wait time | Until completion | Can continue immediately |
| Conversation flow | Interrupted | Smooth |

---

## 🔧 Implementation Checklist

- [ ] Update `.github/copilot-instructions.md` with non-blocking guidelines
- [ ] Update `AGENTS.md` with execution patterns
- [ ] Create helper script: `scripts/run-background.ps1`
- [ ] Add to protocol documentation
- [ ] Test with long-running commands
- [ ] Validate user experience improvement

---

## 📚 Related Documentation

- `.github/copilot-instructions.md` - Agent instructions
- `AGENTS.md` - Agent configuration
- `docs/COMMIT_STANDARD.md` - Commit practices

---

*Protocol Improvement Approved: 2025-12-07*
*Implements: User feedback from v3.1.0 release process*
