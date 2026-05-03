Fix these 3 issues in the current repository. After EACH fix, commit with an appropriate message. Do NOT push.

## Issue #78 - [sevier2][P1] Docker: Missing critical env vars for SEVIER2 endpoints

The Dockerfile does NOT set these critical environment variables that SEVIER2 handlers depend on at runtime:
- XAVIER2_URL (used by verify_save_handler and sync_check_handler)
- X-CORTEX-TOKEN (used by verify_save_handler)

Fix: Add these env vars to the Dockerfile with appropriate defaults/placeholders.

## Issue #75 - [sevier2][P1] Missing /xavier2/agents/{id}/unregister endpoint

AgentRegistry (src/coordination/agent_registry.rs) implements 4 core functions but only 3 have endpoint wiring:
- register -> POST /xavier2/agents/register (wired)
- heartbeat -> POST /xavier2/agents/{id}/heartbeat (wired)
- unregister -> NOT WIRED
- list_agents -> POST /xavier2/agents/list (wired)

Fix: Wire the unregister function to POST /xavier2/agents/{id}/unregister in the router/handler code.

## Issue #74 - [sevier2][P1] Test script sends wrong payload for /xavier2/agents/register

scripts/test-sevier2-endpoints.ps1 sends this payload for agent registration:
agent_id = "powershell-validator-27627459"
agent_type = "validation"  # WRONG - field does not exist in Agent struct

Fix: Check the Agent struct definition in src/coordination/agent_registry.rs and fix the test script to send the correct payload fields (agent_id, name, capabilities, endpoint).

After each fix: git add -A && git commit -m "fix: [description of what was fixed]"
Do NOT push.
