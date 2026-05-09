---
title: "DEPRECATE: Agent Feature Migration to Skills"
type: DEPRECATION
id: "deprecate-agents-skills"
status: OPEN
created: 2026-05-09
priority: HIGH
assignee: "@antigravity"
summary: |
  Deprecated the legacy `.github/agents` and `.github/prompts` architecture which was tied to a specific VS Code extension.
  The ecosystem is migrating to a modular **Skills** architecture located in `.agents/skills`.
---

# ⚠️ Deprecation: Agent Feature Migration to Skills

## Background

The `.github/agents` directory contained `.agent.md` files that defined personas for a legacy VS Code extension. This approach is being replaced by a more modular and framework-agnostic **Skills** paradigm.

## Changes Made

- [x] Deleted `.github/agents/` directory.
- [x] Deleted `.github/prompts/` directory.
- [x] Renamed `.gitcore/AGENT_INDEX.md` to `.gitcore/SKILLS_INDEX.md`.
- [x] Updated `.gitcore/TODO.md` to remove legacy IDE integration items.

## Impacts

- **VS Code Extension:** Any legacy extension relying on these files will no longer function as expected.
- **Agent Protocols:** Agents should now look at `.gitcore/SKILLS_INDEX.md` to identify capabilities and load them from `.agents/skills/`.

## Next Steps

- [ ] Update documentation in `docs/site/` to reflect the skills-based architecture.
- [ ] Ensure all active agents (Bela, Leonardo, etc.) are compatible with the new skill loading mechanism.
- [ ] Remove any remaining legacy scripts that reference the old agent directories.

---
*Reference: User request on 2026-05-09*
