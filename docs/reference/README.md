# 📚 Reference - Information Oriented

> **"Look it up"** - Precise, authoritative technical information

## What is Reference Documentation?

Reference material is **technical description** of Git-Core Protocol. It's dry, factual, and complete.

### When to use reference docs

- ✅ You need to look up exact syntax
- ✅ You need authoritative information
- ✅ You're checking options or parameters
- ✅ You need to verify behavior

### What reference docs are NOT

- ❌ Learning material (see [Tutorials](../tutorials/))
- ❌ Problem-solving guides (see [How-To](../how-to/))
- ❌ Explanations of "why" (see [Explanation](../explanation/))

---

## Available Reference

### 📝 Standards & Conventions

| Document | Content |
|----------|---------|
| **[Commit Standard](./COMMIT_STANDARD.md)** | Extended Conventional Commits format |
| *Label System* (coming soon) | All GitHub labels and their meaning |
| *File Naming* (coming soon) | Conventions for docs, scripts, configs |

### 🛠️ CLI Reference

| Document | Content |
|----------|---------|
| *git-core Commands* (coming soon) | All CLI commands with options |
| *Script Reference* (coming soon) | PowerShell/Bash scripts API |

### 🤖 Agent APIs

| Document | Content |
|----------|---------|
| *Agent YAML Spec* (coming soon) | Custom agent definition format |
| *MCP Tools* (coming soon) | Available Model Context Protocol tools |
| *Frontmatter Schema* (coming soon) | YAML metadata for agent-docs |

### 🔧 Configuration

| Document | Content |
|----------|---------|
| *AGENTS.md Fields* (coming soon) | All configuration options |
| *ARCHITECTURE.md Schema* (coming soon) | Decision table format |
| *Workflow Variables* (coming soon) | GitHub Actions variables |

---

## Reference Principles (Diátaxis)

Our reference docs follow strict rules:

| Principle | Meaning |
|-----------|---------|
| **Information-oriented** | Facts only, no opinions |
| **Structured** | Organized like the product itself |
| **Consistent** | Same format everywhere |
| **Complete** | Every option, every parameter |
| **Accurate** | Source of truth |
| **Neutral** | No instructions, just description |

### Expected Experience

When consulting reference, you should:

- ✅ Find information quickly
- ✅ Get exact, authoritative answers
- ✅ See consistent formatting
- ✅ Trust the accuracy

---

## Reference Format

Reference docs use these patterns:

### Commands

```markdown
## command-name

**Description:** Brief one-line description

**Syntax:**
```

command-name [OPTIONS] <ARGS>

```

**Options:**
- `--option` - Description

**Examples:**
```bash
command-name --option value
```

**See Also:** Links to related commands

```

### Configuration Fields

```markdown
## field_name

**Type:** string | number | boolean | array
**Required:** yes | no
**Default:** value

**Description:** What this field does

**Valid Values:**
- `value1` - When to use
- `value2` - When to use

**Example:**
```yaml
field_name: value
```

```

---

## Need Something Else?

- **I want to learn** → [Tutorials](../tutorials/)
- **I need to solve a problem** → [How-To](../how-to/)
- **I want context or background** → [Explanation](../explanation/)
- **I'm an AI agent** → [Agent Docs](../agent-docs/)

---

*Follow the [Diátaxis framework](https://diataxis.fr/reference/) principles*
