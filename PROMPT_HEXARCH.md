Fix this issue in the current repository. Commit with an appropriate message. Do NOT push.

## Issue #90 - [epic] Hexagonal Architecture Refactor - Untangle Ports from Infrastructure

Xavier2 has hexagonal architecture infrastructure (ports + adapters) but it is completely bypassed. All handlers call domain/infrastructure code directly.

Current state:
- Ports exist in src/ports/ (agent_port, embedding_port, health_check_port, memory_port, pattern_port, security_port, storage_port, time_metrics_port)
- Adapters exist in src/adapters/ (inbound/ and outbound/)
- But handlers bypass ports and call domain/infrastructure directly

Goal: Route all handler calls through the port interfaces instead of direct domain calls.

Steps:
1. Identify which handlers bypass ports
2. Refactor them one by one to use port interfaces
3. Ensure no direct domain calls remain

After fixing: git add -A && git commit -m "refactor: hexagonal architecture - untangle ports from infrastructure"
Do NOT push.
