You are at E:\scripts-python\xavier2 on branch fix/issue-115. All code compiles clean. You have 414 passing lib tests.

Apply these fixes. ONLY modify the files listed. Do NOT touch Gestalt MCP tools (src/server/mcp_server.rs Gestalt-related code), tests/websocket_events.rs, or any other unlisted file.

## Fix 1: Remove hardcoded dev-token defaults (security)

### 1a: src/cli.rs
Find where `dev-token` or `"dev-token"` is used as a default token value. Replace it with an approach that:
- Reads `XAVIER2_TOKEN` env var first
- If not set and running in HTTP mode, generate a random 32-char token and log a WARNING like "XAVIER2_TOKEN not set, generated random token: <token>"
- Store the resolved token in a variable

Look for code like `let token = "dev-token"` or `value_t!(...)` or similar CLI arg defaults.

### 1b: src/adapters/inbound/http/routes.rs
Search for any hardcoded `"dev-token"` string. If it's used as a middleware check (like `if token != "dev-token"`), replace it with checking against the actual resolved token from AppState or a config struct.

### 1c: Dockerfile
Remove `XAVIER2_TOKEN=dev-token` default. Add a comment: `# Required: set XAVIER2_TOKEN to a secure random value`

## Fix 2: Fix e2e test (test_health_endpoint_via_xavier2_binary)

### 2a: Inspect the test
Look at tests/health_endpoint_test.rs or tests/integration.rs for `test_health_endpoint_via_xavier2_binary`.
The test likely spawns `cargo run` or the binary directly and expects `/health` to respond 200.

### 2b: Fix the CLI entrypoint
In src/main.rs or src/cli.rs, ensure that when the binary is run WITHOUT arguments (or with `http` subcommand), it starts the HTTP server that serves `/health`.

The issue is likely that the binary requires `http` as a subcommand but the test invokes it bare.

Fix: If binary is invoked with no arguments and the command is `http` (the default), automatically start the HTTP server with the default config. OR fix the test to pass `http` as an argument.

### 2c: /health route
Ensure src/adapters/inbound/http/routes.rs or src/server/http.rs has a GET /health route that returns 200 with `{"status":"ok"}`.

## Fix 3: Minimax API key from query string to header

### 3a: Find the Minimax provider
Look in src/agents/provider.rs or similar for Minimax API key usage. The Gemini analysis found the key passed in query string.

Change from: `https://api.minimax.com/v1/chat?key=API_KEY`
Change to: `Authorization: Bearer API_KEY` in the request headers.

After making ALL changes, run:
cargo build --lib 2>&1 | Select-Object -Last 5
cargo test --lib 2>&1 | Select-Object -Last 10

Do NOT modify: any Gestalt MCP code in mcp_server.rs, tests/websocket_events.rs, Cargo.toml unless absolutely necessary (and if so, explain why).
