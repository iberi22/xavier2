"""
Multi-Memory Benchmark - Using CLI for Engram, HTTP for Xavier2
============================================================
Compare memory systems using their native interfaces:
- Engram: CLI commands (engram search, engram mem_save)
- Xavier2: HTTP API
"""

import json
import time
import subprocess
import asyncio
import aiohttp
from datetime import datetime
from pathlib import Path

# Engram CLI path
ENGRAM_CLI = "C:\\Users\\belal\\AppData\\Local\\Temp\\engram\\engram.exe"

# Xavier2 HTTP
XAVIER2_URL = "http://localhost:8003"
XAVIER2_TOKEN = "dev-token"

# LOCOMO Queries
QUERIES = [
    {"id": "SH-01", "type": "single_hop", "query": "What is SWAL's default model?"},
    {"id": "SH-02", "type": "single_hop", "query": "What is ManteniApp pricing?"},
    {"id": "SH-03", "type": "single_hop", "query": "Who is Leonardo working with?"},
    {"id": "MH-01", "type": "multi_hop", "query": "Who worked on Cortex and what decisions were made?"},
    {"id": "MH-02", "type": "multi_hop", "query": "Client interested in AI maintenance?"},
    {"id": "TR-01", "type": "temporal", "query": "When was pplx-embed fixed?"},
    {"id": "TR-02", "type": "temporal", "query": "SurrealDB decision date?"},
    {"id": "OD-01", "type": "open_domain", "query": "SWAL operations status?"},
    {"id": "OD-02", "type": "open_domain", "query": "Available skills for sales?"},
]

def log(msg):
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}")

# ============ ENGRAM CLI ============
def engram_search(query: str, timeout: int = 10) -> dict:
    """Search using Engram CLI."""
    start = time.time()
    try:
        result = subprocess.run(
            [ENGRAM_CLI, "search", query],
            capture_output=True,
            text=True,
            timeout=timeout
        )
        elapsed = (time.time() - start) * 1000
        
        if result.returncode == 0:
            return {
                "success": True,
                "latency_ms": elapsed,
                "output": result.stdout,
                "has_results": len(result.stdout.strip()) > 0
            }
        else:
            return {
                "success": False,
                "latency_ms": elapsed,
                "error": result.stderr
            }
    except subprocess.TimeoutExpired:
        return {"success": False, "latency_ms": timeout * 1000, "error": "timeout"}
    except Exception as e:
        return {"success": False, "latency_ms": 0, "error": str(e)}

def engram_save(content: str, topic: str = "benchmark") -> bool:
    """Save memory using Engram CLI."""
    try:
        result = subprocess.run(
            [ENGRAM_CLI, "mem", "save", content, "--topic", topic],
            capture_output=True,
            text=True,
            timeout=10
        )
        return result.returncode == 0
    except:
        return False

def engram_stats() -> dict:
    """Get Engram stats via CLI."""
    try:
        result = subprocess.run(
            [ENGRAM_CLI, "stats"],
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            return {"success": True, "output": result.stdout}
        return {"success": False}
    except:
        return {"success": False}

# ============ XAVIER2 HTTP ============
async def xavier2_search(query: str, timeout: int = 30) -> dict:
    """Search using Xavier2 HTTP API."""
    start = time.time()
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{XAVIER2_URL}/memory/search",
                json={"query": query, "limit": 5},
                headers={"X-Xavier2-Token": XAVIER2_TOKEN},
                timeout=aiohttp.ClientTimeout(total=timeout)
            ) as resp:
                elapsed = (time.time() - start) * 1000
                result = await resp.json() if resp.status == 200 else {}
                return {
                    "success": resp.status == 200,
                    "latency_ms": elapsed,
                    "results": result.get("results", []),
                    "count": len(result.get("results", []))
                }
    except Exception as e:
        return {"success": False, "latency_ms": 0, "error": str(e)}

async def xavier2_health() -> bool:
    """Check Xavier2 health."""
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{XAVIER2_URL}/health",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                return resp.status == 200
    except:
        return False

# ============ BENCHMARK ============
async def run_benchmark():
    log("=" * 60)
    log("MULTI-MEMORY BENCHMARK (CLI + HTTP)")
    log(f"Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    log("=" * 60)
    
    results = {"engram": [], "xavier2": []}
    
    # Pre-check systems
    log("\n[1] System Status")
    log("-" * 40)
    
    # Check Engram
    engram_ok = Path(ENGRAM_CLI).exists()
    log(f"  Engram CLI: {'FOUND' if engram_ok else 'NOT FOUND'}")
    
    # Check Xavier2
    xav_ok = await xavier2_health()
    log(f"  Xavier2 HTTP: {'UP' if xav_ok else 'DOWN'}")
    
    if not engram_ok:
        log("\n  ERROR: Engram CLI not found at expected path")
        log(f"  Expected: {ENGRAM_CLI}")
    
    if not xav_ok:
        log("\n  ERROR: Xavier2 not responding on {XAVIER2_URL}")
    
    # Run benchmark
    log("\n[2] Running Benchmark Queries")
    log("-" * 40)
    
    for q in QUERIES:
        log(f"\n  [{q['id']}] {q['query'][:50]}...")
        
        # Engram CLI
        if engram_ok:
            r = engram_search(q["query"])
            results["engram"].append({
                "id": q["id"],
                "type": q["type"],
                "latency_ms": r["latency_ms"],
                "success": r["success"],
                "has_results": r.get("has_results", False)
            })
            status = "OK" if r["success"] else "FAIL"
            log(f"    Engram CLI: {status} ({r['latency_ms']:.0f}ms)")
        else:
            results["engram"].append({
                "id": q["id"],
                "type": q["type"],
                "latency_ms": 0,
                "success": False,
                "has_results": False
            })
        
        # Xavier2 HTTP
        if xav_ok:
            r = await xavier2_search(q["query"])
            results["xavier2"].append({
                "id": q["id"],
                "type": q["type"],
                "latency_ms": r["latency_ms"],
                "success": r["success"],
                "count": r.get("count", 0)
            })
            status = "OK" if r["success"] else "FAIL"
            count = r.get("count", 0)
            log(f"    Xavier2 HTTP: {status} ({r['latency_ms']:.0f}ms, {count} results)")
        else:
            results["xavier2"].append({
                "id": q["id"],
                "type": q["type"],
                "latency_ms": 0,
                "success": False,
                "count": 0
            })
    
    # Summary
    log("\n" + "=" * 60)
    log("SUMMARY")
    log("=" * 60)
    
    for system, data in results.items():
        if not data:
            continue
            
        successful = sum(1 for r in data if r["success"] and r.get("has_results", r.get("count", 0)) > 0)
        latencies = [r["latency_ms"] for r in data if r["success"]]
        avg_lat = sum(latencies) / len(latencies) if latencies else 0
        
        recall = successful / len(QUERIES) * 100 if QUERIES else 0
        
        log(f"\n  {system.upper()}:")
        log(f"    Recall: {successful}/{len(QUERIES)} ({recall:.1f}%)")
        log(f"    Avg Latency: {avg_lat:.0f}ms")
        
        if latencies:
            log(f"    Min/Max: {min(latencies):.0f}ms / {max(latencies):.0f}ms")
        
        # By type
        by_type = {}
        for r in data:
            t = r["type"]
            if t not in by_type:
                by_type[t] = {"count": 0, "total": 0}
            by_type[t]["count"] += int(r.get("has_results", r.get("count", 0)) > 0)
            by_type[t]["total"] += 1
        
        log("    By Type:")
        for t, stats in sorted(by_type.items()):
            pct = stats["count"] / stats["total"] * 100 if stats["total"] else 0
            log(f"      {t}: {stats['count']}/{stats['total']} ({pct:.0f}%)")
    
    # Save
    output_dir = Path("E:/scripts-python/xavier2/benchmark_results")
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / f"multi_benchmark_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    
    with open(output_file, "w") as f:
        json.dump({
            "timestamp": datetime.now().isoformat(),
            "systems": {
                "engram": {"type": "cli", "path": ENGRAM_CLI if engram_ok else None},
                "xavier2": {"type": "http", "url": XAVIER2_URL}
            },
            "queries": len(QUERIES),
            "results": results
        }, f, indent=2)
    
    log(f"\nSaved: {output_file}")
    
    return results

if __name__ == "__main__":
    asyncio.run(run_benchmark())
