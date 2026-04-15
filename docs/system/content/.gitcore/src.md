# SRC - Source Code Reference

## Overview

This document provides a comprehensive reference to the Xavier2 source code structure. It serves as the **Source Recipe** for all engineering and development tasks.

---

## Directory Structure

```
xavier2/src/
├── a2a/              # Agent-to-Agent protocol implementation
├── agents/           # Agent definitions and behaviors
├── checkpoint/       # State checkpointing and recovery
├── coordination/     # Multi-agent coordination logic
├── memory/           # Memory systems (vector, graph, episodic)
├── scheduler/        # Task scheduling and queue management
├── secrets/          # Secret management and encryption
├── security/         # Security primitives
├── server/           # HTTP/gRPC server implementation
├── tasks/            # Task definitions and execution
├── tools/            # Tool definitions and integrations
├── ui/               # User interface components
├── lib.rs            # Library root
├── main.rs           # Binary entry point
├── main_egui.rs     # Egui UI binary
└── web.rs            # Web server binary
```

---

## Core Modules

### a2a/ - Agent-to-Agent Protocol

**Purpose:** Protocol for communication between agents.

**Key Files:**
- `mod.rs` - Module definition
- `server.rs` - A2A server implementation
- `client.rs` - A2A client
- `protocol.rs` - Protocol definitions
- `types.rs` - Type definitions

**Usage:**
```rust
use xavier2::a2a::{A2AServer, A2AClient};
```

---

### agents/ - Agent Definitions

**Purpose:** Agent behaviors, traits, and implementations.

**Key Files:**
- `mod.rs` - Agent trait definitions
- `executor.rs` - Agent execution logic
- `swarm.rs` - Agent swarm management
- `memory.rs` - Agent memory integration

**Usage:**
```rust
use xavier2::agents::{Agent, Swarm};
```

---

### memory/ - Memory Systems

**Purpose:** Vector search, graph storage, episodic memory.

**Key Files:**
- `mod.rs` - Memory trait definitions
- `vector.rs` - Vector store implementation
- `graph.rs` - Belief graph
- `episodic.rs` - Episodic memory
- `surreal.rs` - SurrealDB integration
- `hybrid.rs` - Hybrid search (BM25 + Vector)

**Usage:**
```rust
use xavier2::memory::{Memory, VectorStore, BeliefGraph};
```

---

### server/ - HTTP Server

**Purpose:** REST and gRPC server for Xavier2.

**Key Files:**
- `mod.rs` - Server setup
- `routes.rs` - HTTP routes
- `websocket.rs` - WebSocket support
- `mcp.rs` - Model Context Protocol server

**Usage:**
```rust
use xavier2::server::Server;
```

---

### tasks/ - Task Management

**Purpose:** Task definitions, execution, and tracking.

**Key Files:**
- `mod.rs` - Task trait
- `queue.rs` - Task queue
- `executor.rs` - Task executor
- `scheduler.rs` - Task scheduler

---

### coordination/ - Multi-Agent Coordination

**Purpose:** Coordination between multiple agents.

**Key Files:**
- `mod.rs` - Coordination logic
- `messages.rs` - Coordination messages
- `leader.rs` - Leader election
- `consensus.rs` - Consensus algorithms

---

## Important Types

### Agent Trait

```rust
pub trait Agent {
    fn id(&self) -> String;
    fn name(&self) -> String;
    async fn run(&mut self, context: &Context) -> Result<Response, Error>;
    fn reset(&mut self);
}
```

### Memory Trait

```rust
pub trait Memory {
    async fn store(&self, entry: MemoryEntry) -> Result<String, Error>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, Error>;
    async fn delete(&self, id: &str) -> Result<(), Error>;
}
```

### Task Trait

```rust
pub trait Task: Send + Sync {
    fn id(&self) -> String;
    fn execute(&self) -> impl Future<Output = Result<TaskResult, Error>>;
}
```

---

## Build & Run

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With all features
cargo build --all-features
```

### Run

```bash
# Default server
cargo run --bin xavier2

# With custom config
cargo run --bin xavier2 -- --config config.yaml
```

---

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --lib memory

# Run with coverage
cargo tarpaulin --out html
```

---

## Dependencies

### Key Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| surrealdb | ^1.0 | Vector + Graph DB |
| tokio | ^1.0 | Async runtime |
| tantivy | ^0.22 | BM25 search |
| serde | ^1.0 | Serialization |
| axum | ^0.7 | HTTP server |
| tower | ^0.4 | Middleware |

---

## CLI Reference

```bash
# Start server
xavier2 server start

# Run agent
xavier2 agent run <agent_id>

# Memory operations
xavier2 memory search <query>
xavier2 memory store <content>

# Task operations
xavier2 task list
xavier2 task execute <task_id>
```

---

*Document version: 1.0*
*Last updated: 2026-03-15*
*Part of GitCore Protocol*
