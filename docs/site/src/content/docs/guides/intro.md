---
title: Introduction
description: Current overview of Xavier2
---

# Xavier2 Documentation

> **Xavier2** is a Rust memory runtime for AI agent workflows.

For local operation, the recommended interface is authenticated HTTP with `curl`. MCP remains available as an optional transport for IDE-native tooling.

## What Xavier2 Currently Provides

- **Shared memory storage** for agent workflows
- **Semantic retrieval** through embeddings and indexed memory
- **Belief graph support** in the runtime and memory modules
- **Workspace-aware usage tracking** and quotas
- **Code indexing** and symbol search through `code-graph`
- **Panel UI** and **optional MCP transport**

## Current Runtime Shape

The current integrated binary exposes:

- authenticated HTTP routes
- optional MCP transport
- panel UI routes
- code indexing and symbol search
- workspace-aware memory and usage tracking

## Current Storage Story

- Default validated backend: `FileMemoryStore`
- SurrealDB is present in the codebase and Docker setup
- SurrealDB should currently be treated as optional or future-facing, not as the default validated backend

## Current Product Positioning

The most accurate public description today is:

- open source memory runtime for agent workflows
- internally validated production candidate

Avoid stronger claims until latency, auth hardening, and monitoring are closed operationally.

## Quick Links

- [Installation Guide](/guides/installation/)
- [Quick Start](/guides/quick-start/)
- [Architecture Overview](/architecture/overview/)
- [API Reference](/reference/api/)
- [Testing Overview](/testing/overview/)
