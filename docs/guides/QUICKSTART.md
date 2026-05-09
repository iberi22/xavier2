# Quick Start Guide

## Installation

### From Source

\\\ash
git clone https://github.com/iberi22/xavier.git
cd xavier
cargo build --release
./target/release/xavier http
\\\

### From Binary

Download from GitHub Releases for your platform.

### Docker

\\\ash
docker run -p 8006:8006 ghcr.io/iberi22/xavier:latest
\\\

## First Steps

1. **Start the server:**
   \\\ash
   xavier http
   \\\

2. **Add your first memory:**
   \\\ash
   xavier add "Hello Xavier!" --title "First Memory"
   \\\

3. **Search:**
   \\\ash
   xavier search "hello"
   \\\

4. **Check stats:**
   \\\ash
   xavier stats
   \\\

## Next Steps

- [CLI Reference](./CLI_REFERENCE.md) - Full CLI documentation
- [API Reference](../reference/API.md) - HTTP API details
- [MCP Integration](./MCP_INTEGRATION.md) - Connect to AI clients
