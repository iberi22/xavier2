# ⚙️ CLI Orchestration Structure

This file defines the configuration and dependency graph for the Git-Core CLI ecosystem.

## 📦 Dependency Graph

| CLI Tool | Required Version | Provider | Priority |
|----------|------------------|----------|----------|
| `git` | >= 2.40.0 | System | Critical |
| `gh` | >= 2.40.0 | GitHub | Critical |
| `gc` | >= 0.1.0 | Git-Core | Critical |
| `gemini` | >= 1.0.0 | Google | High |
| `copilot`| latest | GitHub | Medium |
| `jules` | >= 0.5.0 | Internal | Medium |

## 🔗 Capability Map

Agents should consult this map to determine which tool to use for a specific capability.

### 🧠 Intelligence
- **Deep Reasoning/Context:** `gemini`
- **Code Completion/snippets:** `copilot` / `gh copilot`
- **Refactoring:** `copilot` / `gemini`

### 🏗️ Operations
- **Task Management:** `gh issue` / `gc issue`
- **Version Control:** `gc` (wrapper) > `git` (native)
- **CI/CD:** `gh run` / `act` (if installed)

## 🛠️ Configuration Overrides

### `gc init` Behavior
- **Interactive:** `true` (Agent must handle prompts if strictly necessary, or usage `--yes` / `--auto` flags)
- **Dependency Strictness:** `Warn` (Allow proceed if minor tools missing, specific user prompt required).

### Agent Permissions
- **Install Global:** ❌ Forbidden (Agents cannot run `npm install -g` without approval)
- **Install Local:** ✅ Allowed ( `npm install`)
- **Config Set:** ✅ Allowed (`git config`, `gh config`)

## 🤖 Automations
- `gc-cli` automatically invokes `gh` for auth checks.
- `jules` uses `gc` for state reporting.
