---
title: "Advanced Tool Calling Patterns"
type: SPEC
id: "spec-advanced-tool-calling"
created: 2025-12-14
updated: 2025-12-14
agent: system
model: general
requested_by: user
summary: |
  Details the advanced patterns for agent tool calling, including tool search, programmatic calls, and secure bridge architecture.
keywords: [tool, calling, patterns, search, bridge, architecture, tokens]
tags: ["#tools", "#optimization", "#security"]
project: Git-Core-Protocol
---

# 🛠️ Advanced Tool Calling Specification

This document details four advanced tool calling patterns integrated into Git-Core Protocol to optimize AI agent performance, improve context window utilization, and ensure security.

## 1. Tool Search (Búsqueda de herramientas)

### The Problem
Defining a large number of tools upfront consumes a significant portion of the LLM context window. This happens before the agent even begins reasoning about the actual task, increasing both token cost and latency.

### The Solution
Implement a dynamic loading mechanism via a `tool_search` function.
Instead of sending 50 tool schemas, the LLM starts with only `tool_search`.
When it receives a task, it uses `tool_search` to discover the necessary tools, loading their schemas on-demand. This pattern drastically minimizes context usage and keeps the active prompt tightly focused on the current objective.

## 2. Programmatic Tool Calling (Llamada programática a herramientas)

### The Problem
Iterating over large datasets (e.g., verifying budget compliance for 100 team members) by calling a tool repetitively (100 sequential LLM calls) is extremely slow and causes repetitive hallucinations and compounding context growth.

### The Solution
Use **Programmatic Tool Calling**.
The LLM writes an orchestrator script (e.g., in Python or Bash) inside the execution sandbox. This script contains a loop that programmatically iterates over the dataset and triggers the underlying actions or sub-tools.
- **Benefits:** Massive token savings, zero context drift during iterations, and high accuracy for batch operations.

## 3. Secure Tool Bridge Architecture (Arquitectura de puente)

### The Problem
Agents often need tools that perform sensitive actions (like DB updates or interacting with internal APIs) which cannot securely reside in an internet-isolated sandbox environment.

### The Solution
The code execution sandbox cannot access the internet directly, nor does it contain any API credentials.
Instead, we use a **Tool Bridge**.
When the agent executes a script in the sandbox calling a tool, the call is serialized and securely routed via a bridge protocol to the backend Python/Rust application, which possesses the credentials and executes the action on the agent's behalf.
- **Benefits:** Prevents credential exfiltration and strictly governs what actions a compromised agent can perform.

## 4. Tool Usage Examples (Ejemplos de uso de herramientas)

### The Problem
LLMs frequently struggle to correctly format arguments for complex tools, resulting in parsing errors or incorrect actions (with historical accuracy hovering around 72% for highly complex tools).

### The Solution
Inject comprehensive, concrete usage examples into the tool definitions.
Examples teach the LLM the exact syntax and expected data shapes. Metrics show that rich tool examples increase accurate tool parameter construction from 72% to over 90%.

---
