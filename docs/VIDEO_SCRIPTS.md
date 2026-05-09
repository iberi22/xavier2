# Video Scripts

## Video 1: Xavier Introduction

### Intro

- Visual: Xavier logo and tagline
- Line: "Welcome to Xavier, the cognitive memory layer for AI agent workflows."

### What Xavier Is

- Explain memory, belief graph, and MCP access
- Show the difference between task tracking and durable memory
- Position Xavier as infrastructure for long-running agents

### Main Features

- Hybrid search
- Belief graph relationships
- MCP server surface
- System 1 / 2 / 3 execution model

### Demo

- Health check
- Add memory
- Search memory

## Video 2: Quick Start

### Prerequisites

- Rust
- Docker

### Install and Run

```bash
git clone https://github.com/southwest-ai-labs/xavier.git
cd xavier
docker compose up -d
cargo test
```

### First Calls

```bash
curl http://localhost:8003/health
curl -X POST http://localhost:8003/memory/add ...
curl -X POST http://localhost:8003/memory/search ...
```

## Video 3: Memory and Belief Graph

### Focus

- Hybrid retrieval
- Metadata and tagging
- Relationship tracking
- Long-horizon memory reuse

## Video 4: Git-Core and IDE Integration

### Focus

- Xavier as shared memory backend
- GitHub Issues as task state
- MCP configuration in IDEs
- Antigravity, Copilot, Cursor, and Windsurf alignment

## Video 5: Production Deployment

### Focus

- Docker deployment
- Security and token handling
- Monitoring and logs
- Backup and recovery patterns
