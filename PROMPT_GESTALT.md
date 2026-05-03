Fix these 2 issues in the current repository. After EACH fix, commit with an appropriate message. Do NOT push.

## Issue #149 - [MIGRATION] Gestalt Rust → Xavier2: Unificar memoria como backend único

Currently Gestalt Rust and Xavier2 are two separate projects that both use SurrealDB as memory backend, each with their own client implementation. This creates duplication and dual maintenance.

Goal: Make Gestalt Rust use Xavier2 as its exclusive memory backend, eliminating the Gestalt memory system entirely.

Steps:
1. Identify what memory-related code exists in Gestalt Rust (examine any Gestalt-related files/crates in this repo)
2. Map Gestalt's memory operations to Xavier2's existing MemoryPort/adapters
3. Remove or redirect Gestalt memory code to delegate to Xavier2

## Issue #166 - MCP protocol handler for Gestalt MemoryFragment

See docs/JULES_PROMPTS_MAY2026.md Prompt 2 for instructions.

Since Jules already created PR #168 with a partial MCP Gestalt implementation (save_fragment, search_fragments, get_recent_fragments, security scanning via SecurityScanPort), you need to:
1. Review what PR #168 implemented (check git log for feats from Jules)
2. Complete any remaining pieces
3. Ensure proper integration with the memory system

After each fix: git add -A && git commit -m "fix: [description]"
Do NOT push.
