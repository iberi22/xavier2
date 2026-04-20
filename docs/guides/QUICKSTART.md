# Quick Start Guide

## Installation

### From Source

\\\ash
git clone https://github.com/iberi22/xavier2-1.git
cd xavier2-1
cargo build --release
./target/release/xavier2 http
\\\

### From Binary

Download from GitHub Releases for your platform.

### Docker

\\\ash
docker run -p 8006:8006 ghcr.io/iberi22/xavier2:latest
\\\

## First Steps

1. **Start the server:**
   \\\ash
   xavier2 http
   \\\

2. **Add your first memory:**
   \\\ash
   xavier2 add "Hello Xavier2!" --title "First Memory"
   \\\

3. **Search:**
   \\\ash
   xavier2 search "hello"
   \\\

4. **Check stats:**
   \\\ash
   xavier2 stats
   \\\

## Next Steps

- [CLI Reference](./CLI_REFERENCE.md) - Full CLI documentation
- [API Reference](../reference/API.md) - HTTP API details
- [MCP Integration](./MCP_INTEGRATION.md) - Connect to AI clients
