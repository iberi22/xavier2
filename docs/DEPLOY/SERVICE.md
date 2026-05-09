# Xavier Service Management

## Overview

Xavier can run as a reliable background service with self-healing, graceful shutdown, and process monitoring.

## Quick Start

```powershell
cd E:\scripts-python\xavier

# Start the service
.\scripts\xavier-service.ps1 start

# Check status
.\scripts\xavier-service.ps1 status

# View logs
.\scripts\xavier-service.ps1 logs

# Restart
.\scripts\xavier-service.ps1 restart

# Stop
.\scripts\xavier-service.ps1 stop

# Install as auto-start (Windows Task Scheduler)
.\scripts\xavier-service.ps1 install
```

## Service Script Features

- **Auto-restart**: Detects crashes and restarts within 5 seconds
- **Port conflict detection**: Identifies and resolves port conflicts before starting
- **Log rotation**: Max 5MB per file, keeps 5 backups
- **Health monitoring**: Polls `/health` endpoint every 15s, triggers restart if watchdog times out (60s)
- **Graceful shutdown**: SIGTERM/SIGINT properly handled with connection draining

## Health Endpoints

| Endpoint | Purpose | Auth |
|----------|---------|------|
| `GET /health` | Liveness probe — process is alive | None |
| `GET /readiness` | Readiness probe — all dependencies ready | None |
| `GET /build` | Build info, memory backend, config | None |

### Health Response

```bash
curl http://localhost:8040/health
# {"status":"ok","service":"xavier","version":"0.4.1"}
```

### Readiness Response

```bash
curl http://localhost:8040/readiness
# Returns detailed status of: workspace, memory_store, code_graph, embeddings, llm
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER_PORT` | `8040` | HTTP server port |
| `XAVIER_TOKEN` | `dev-token` | Auth token |
| `RUST_LOG` | `info` | Log filter (trace,debug,info,warn,error) |
| `XAVIER_LOG_LEVEL` | `info` | Alternative log filter |
| `XAVIER_MEMORY_BACKEND` | `vec` | Memory backend: `vec`, `surrealdb`, `auto` |
| `XAVIER_CODE_GRAPH_DB_PATH` | `data/code_graph.db` | Code graph DB path |

## Graceful Shutdown

Xavier handles the following shutdown signals:

- **SIGTERM / SIGINT**: Connection draining then exit
- **Ctrl+C**: Same as SIGTERM on Windows
- **Ctrl+Break**: Force exit (no drain)

Shutdown sequence:
1. Signal received → stop accepting new connections
2. Wait up to 10s for in-flight requests to complete
3. Exit

## Panic Recovery

Panics are caught and logged with:
- Timestamp
- Thread name
- File:line:column location
- Message payload
- Stack trace

Panics are never silently swallowed.

## Architecture

```
┌─────────────────────────────────────────┐
│   xavier-service.ps1 (wrapper)        │
│   - PID tracking (data/xavier.pid)   │
│   - Health watchdog (60s timeout)      │
│   - Auto-restart on crash               │
│   - Port conflict resolution             │
│   - Log rotation                        │
└────────────┬────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│   xavier.exe server                    │
│   - /health  (liveness)                │
│   - /readiness (readiness)              │
│   - SIGTERM/SIGINT handling             │
│   - Panic hook (logs to stderr)         │
│   - Graceful connection drain            │
└─────────────────────────────────────────┘
```

## Troubleshooting

### Port already in use

```powershell
# Find what's using port 8040
netstat -ano | findstr :8040

# Kill it
taskkill /PID <PID> /F
```

### Process won't start

```powershell
# Check logs
.\scripts\xavier-service.ps1 logs

# Check binary exists
Test-Path E:\scripts-python\xavier\target\release\xavier.exe

# Build if needed
cd E:\scripts-python\xavier
cargo build --release
```

### Health endpoint not responding

```powershell
# Check if xavier process is running
.\scripts\xavier-service.ps1 status

# Check port is bound
netstat -ano | findstr :8040

# View stderr logs
Get-Content logs\stderr.log -Tail 30
```

## Kubernetes / Container Deployment

For K8s deployments, the service script handles:

- Liveness: `GET /health` — kubelet polls every 10s, restart if failing 3 times
- Readiness: `GET /readiness` — kubelet routes traffic only when all components ready
- Graceful shutdown: 30s termination grace period (`terminationGracePeriodSeconds: 30`)

Example K8s probe config:
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8040
  initialDelaySeconds: 5
  periodSeconds: 10
  failureThreshold: 3
readinessProbe:
  httpGet:
    path: /readiness
    port: 8040
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 3
```

## Auto-Start on Windows

Install via Task Scheduler:
```powershell
.\scripts\xavier-service.ps1 install
```

This creates a task that starts Xavier at system boot under your user account.

Uninstall:
```powershell
.\scripts\xavier-service.ps1 uninstall
```
