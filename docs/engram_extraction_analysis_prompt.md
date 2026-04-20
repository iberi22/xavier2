# Codex Analysis Task: Engram vs Xavier2/Cortex

You are reviewing the local memory stack for a solo-founder coding-agent workflow.

Repositories and artifacts:
- Xavier2 core repo: `E:\scripts-python\xavier2`
- Cortex enterprise/security repo: `E:\scripts-python\cortex`
- Engram binary: `C:\Users\belal\AppData\Local\Temp\engram\engram.exe`

Current product direction:
- Xavier2 is the open-source core: simpler, fast, local-first, designed for one human coordinating many bots.
- Cortex is the enterprise/security shell: Anticipator-based security, confidentiality, permissions, audit, governance, RBAC, tenants.
- Engram is no longer assumed to be a runtime dependency. Treat it as a source of ideas to inspect and possibly absorb.

Analyze:
1. What Engram appears to do well from its CLI/help/docs/runtime behavior.
2. Which Engram ideas are worth extracting into Xavier2.
3. Which Engram ideas belong in Cortex instead.
4. Which Engram capabilities should be ignored because they duplicate or weaken the architecture.
5. Concrete implementation plan with prioritized issues.
6. Risks, especially around privacy, token burn, agent session indexing, and unnecessary dependencies.

Constraints:
- Do not expose secrets.
- Do not modify files.
- Prefer commands like `engram.exe --help`, `engram.exe <subcommand> --help`, repo search, and source inspection.
- Keep output concise and actionable.

Write final answer as a report with:
- Executive recommendation
- Findings
- Extraction candidates for Xavier2
- Extraction candidates for Cortex
- Do-not-adopt list
- Prioritized implementation plan
- Open questions
