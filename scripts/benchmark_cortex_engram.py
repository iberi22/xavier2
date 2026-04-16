"""
SWAL Memory Benchmark - Cortex vs Engram
========================================
Compare the 2 running memory systems with project indexing
"""

import asyncio
import json
import time
from datetime import datetime
from pathlib import Path

# Only 2 systems are reliably running
SYSTEMS = {
    "cortex": {
        "url": "http://localhost:8003",
        "add_endpoint": "/memory/add",
        "search_endpoint": "/memory/search"
    },
    "engram": {
        "url": "http://localhost:7437",
        "add_endpoint": "/prompts",
        "search_endpoint": "/mem/search"
    }
}

PROJECTS = [
    "xavier2", "cortex", "synapse-agentic", "gestalt-rust", "manteniapp",
    "agents-flows-recipes", "edge-hive", "worldexams", "tripro_landing_page_astro",
    "domus-otec",
]

def index_projects(base_path="E:\\scripts-python"):
    """Index project codebases."""
    print("\n=== INDEXING PROJECTS ===\n")
    results = {}
    
    for project in PROJECTS:
        path = Path(base_path) / project
        if not path.exists():
            continue
        
        file_count = 0
        total_lines = 0
        types = {}
        
        for ext in ['*.rs', '*.py', '*.ts', '*.js', '*.md', '*.toml', '*.json']:
            for f in path.rglob(ext):
                if '.git' in str(f) or 'node_modules' in str(f):
                    continue
                try:
                    lines = len(f.read_text(encoding='utf-8', errors='ignore').splitlines())
                    total_lines += lines
                    file_count += 1
                    e = ext[2:]
                    types[e] = types.get(e, 0) + 1
                except:
                    pass
        
        if file_count > 0:
            results[project] = {"files": file_count, "lines": total_lines, "types": types}
            print(f"  {project}: {file_count} files, {total_lines} lines")
    
    return results

async def store(system, config, data):
    """Store data in system."""
    import aiohttp
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                config["url"] + config["add_endpoint"],
                json=data,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                return resp.status in [200, 201]
    except Exception as e:
        print(f"    Error: {e}")
        return False

async def search(system, config, query):
    """Search in system."""
    import aiohttp
    start = time.time()
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                config["url"] + config["search_endpoint"],
                json={"query": query},
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                elapsed = (time.time() - start) * 1000
                return {"success": resp.status == 200, "latency_ms": elapsed}
    except Exception as e:
        return {"success": False, "latency_ms": 0}

async def main():
    print("=" * 60)
    print("SWAL MEMORY BENCHMARK - CORTEX vs ENGRAM")
    print(f"Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 60)
    
    # Index projects
    projects = index_projects()
    
    # Create sync payload
    sync_data = {
        "type": "project_index",
        "timestamp": datetime.now().isoformat(),
        "projects": list(projects.keys()),
        "summary": {
            "total": len(projects),
            "files": sum(p["files"] for p in projects.values()),
            "lines": sum(p["lines"] for p in projects.values())
        }
    }
    
    print("\n=== SYNCING TO SYSTEMS ===\n")
    for name, config in SYSTEMS.items():
        ok = await store(name, config, sync_data)
        print(f"  {name}: {'✅' if ok else '❌'}")
    
    await asyncio.sleep(1)
    
    # Benchmark queries
    queries = [
        "How many projects are indexed?",
        "What is xavier2 status?",
        "Total lines of code across projects",
        "Tell me about ManteniApp",
        "Memory systems architecture",
    ]
    
    print("\n=== BENCHMARK QUERIES ===\n")
    results = {}
    
    for i, q in enumerate(queries):
        print(f"  Q{i+1}: {q[:50]}...")
        results[q] = {}
        for name, config in SYSTEMS.items():
            r = await search(name, config, q)
            results[q][name] = r
            status = "✅" if r["success"] else "❌"
            print(f"    {name}: {status} ({r['latency_ms']:.0f}ms)")
    
    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    for name in SYSTEMS:
        latencies = [results[q][name]["latency_ms"] for q in queries if results[q][name]["success"]]
        avg = sum(latencies) / len(latencies) if latencies else 0
        success = sum(1 for q in queries if results[q][name]["success"]) / len(queries) * 100
        print(f"\n  {name.upper()}")
        print(f"    Avg latency: {avg:.0f}ms")
        print(f"    Success: {success:.0f}%")
    
    print("\n" + "=" * 60)
    
    # Save results
    output = Path("E:\\scripts-python\\xavier2\\benchmark_results")
    output.mkdir(parents=True, exist_ok=True)
    fname = output / f"benchmark_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    fname.write_text(json.dumps({
        "timestamp": datetime.now().isoformat(),
        "systems": list(SYSTEMS.keys()),
        "projects_indexed": len(projects),
        "results": results
    }, indent=2))
    print(f"\n💾 Saved: {fname}")

if __name__ == "__main__":
    asyncio.run(main())
