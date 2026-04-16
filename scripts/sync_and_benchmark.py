"""
SWAL Memory Benchmark - Full Sync + Compare
============================================
1. Index all project codebases
2. Store same data in all 3 memory systems
3. Run benchmark queries
4. Compare results
"""

import asyncio
import json
import time
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any

# Memory Systems Configuration
SYSTEMS = {
    "cortex": {
        "url": "http://localhost:8003",
        "add_endpoint": "/memory/add",
        "search_endpoint": "/memory/search"
    },
    "xavier2": {
        "url": "http://localhost:8006", 
        "add_endpoint": "/memory/add",
        "search_endpoint": "/memory/search"
    },
    "engram": {
        "url": "http://localhost:7437",
        "add_endpoint": "/prompts",
        "search_endpoint": "/mem/search"
    }
}

# Projects to index
PROJECTS = [
    "xavier2",
    "cortex", 
    "synapse-agentic",
    "gestalt-rust",
    "manteniapp",
    "agents-flows-recipes",
    "edge-hive",
    "worldexams",
    "tripro_landing_page_astro",
    "domus-otec",
]

def index_projects(base_path: str = "E:\\scripts-python") -> Dict[str, Any]:
    """Index all project codebases and return summary."""
    print("\n" + "="*60)
    print("INDEXING PROJECTS")
    print("="*60)
    
    results = {}
    for project in PROJECTS:
        project_path = Path(base_path) / project
        if not project_path.exists():
            print(f"  ⚠️  {project}: NOT FOUND")
            continue
            
        try:
            # Count files by extension
            file_counts = {}
            total_lines = 0
            file_count = 0
            
            for ext in ['*.rs', '*.py', '*.ts', '*.js', '*.md', '*.toml', '*.json']:
                for f in project_path.rglob(ext):
                    if '.git' in str(f) or 'node_modules' in str(f):
                        continue
                    try:
                        lines = len(f.read_text(encoding='utf-8', errors='ignore').splitlines())
                        total_lines += lines
                        file_count += 1
                        ext_name = ext[2:]  # Remove *
                        file_counts[ext_name] = file_counts.get(ext_name, 0) + 1
                    except:
                        pass
            
            results[project] = {
                "files": file_count,
                "lines": total_lines,
                "types": file_counts,
                "path": str(project_path)
            }
            print(f"  ✅ {project}: {file_count} files, {total_lines} lines")
        except Exception as e:
            print(f"  ❌ {project}: {e}")
    
    return results

async def store_in_system(system_name: str, config: Dict, data: Dict) -> Dict:
    """Store data in a memory system."""
    import aiohttp
    
    url = config["url"] + config["add_endpoint"]
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                url,
                json=data,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                return {
                    "success": resp.status in [200, 201],
                    "status": resp.status,
                    "body": await resp.json() if resp.status in [200, 201] else None
                }
    except Exception as e:
        return {"success": False, "error": str(e)}

async def search_system(system_name: str, config: Dict, query: str) -> Dict:
    """Search in a memory system."""
    import aiohttp
    
    url = config["url"] + config["search_endpoint"]
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                url,
                json={"query": query},
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                result = await resp.json() if resp.status == 200 else {}
                return {
                    "success": resp.status == 200,
                    "results": result
                }
    except Exception as e:
        return {"success": False, "error": str(e)}

async def sync_all_systems(project_index: Dict[str, Any]):
    """Store project index in all 3 memory systems."""
    print("\n" + "="*60)
    print("SYNCING ALL SYSTEMS WITH PROJECT DATA")
    print("="*60)
    
    # Create summary data
    sync_data = {
        "type": "project_index",
        "timestamp": datetime.now().isoformat(),
        "projects": {},
        "summary": {
            "total_projects": len(project_index),
            "total_files": sum(p["files"] for p in project_index.values()),
            "total_lines": sum(p["lines"] for p in project_index.values())
        }
    }
    
    for project, info in project_index.items():
        sync_data["projects"][project] = {
            "files": info["files"],
            "lines": info["lines"],
            "types": info["types"]
        }
    
    # Store in all systems
    for system_name, config in SYSTEMS.items():
        print(f"\n  📤 Syncing to {system_name}...")
        result = await store_in_system(system_name, config, sync_data)
        if result["success"]:
            print(f"     ✅ {system_name}: synced")
        else:
            print(f"     ❌ {system_name}: {result.get('error', 'Unknown error')}")

async def run_benchmark_queries():
    """Run benchmark queries across all systems."""
    print("\n" + "="*60)
    print("RUNNING BENCHMARK QUERIES")
    print("="*60)
    
    queries = [
        {
            "name": "project_count",
            "query": "How many projects are in the SWAL ecosystem?",
            "expected": "10 projects"
        },
        {
            "name": "xavier2_status", 
            "query": "What is the status of xavier2 project?",
            "expected": "xavier2"
        },
        {
            "name": "codebase_stats",
            "query": "Show me the total lines of code across all projects",
            "expected": "lines"
        },
        {
            "name": "manteniapp",
            "query": "What do you know about ManteniApp?",
            "expected": "ManteniApp"
        },
        {
            "name": "architecture",
            "query": "What is the architecture of the memory systems?",
            "expected": "cortex, xavier2, engram"
        }
    ]
    
    all_results = {}
    
    for qi, q in enumerate(queries):
        print(f"\n  📋 Query {qi+1}: {q['name']}")
        print(f"     Q: {q['query']}")
        
        query_results = {}
        for system_name, config in SYSTEMS.items():
            start = time.time()
            result = await search_system(system_name, config, q["query"])
            elapsed = (time.time() - start) * 1000
            
            query_results[system_name] = {
                "success": result["success"],
                "latency_ms": elapsed,
                "has_results": bool(result.get("results")),
                "result_keys": list(result.get("results", {}).keys())[:3] if result.get("results") else []
            }
            
            status = "✅" if result["success"] else "❌"
            print(f"     {status} {system_name}: {elapsed:.0f}ms")
        
        all_results[q["name"]] = query_results
    
    return all_results

async def main():
    print("="*60)
    print("SWAL MEMORY BENCHMARK - FULL SYNC + COMPARE")
    print(f"Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("="*60)
    
    # Step 1: Index projects
    project_index = index_projects()
    
    # Step 2: Sync to all systems
    await sync_all_systems(project_index)
    
    # Step 3: Run benchmark queries
    await asyncio.sleep(1)  # Brief pause
    benchmark_results = await run_benchmark_queries()
    
    # Step 4: Print summary
    print("\n" + "="*60)
    print("BENCHMARK SUMMARY")
    print("="*60)
    
    print(f"\n{'System':<12} {'Avg Latency':<15} {'Success Rate':<15} {'Status'}")
    print("-" * 60)
    
    for system_name in SYSTEMS:
        results = [benchmark_results[q][system_name] for q in benchmark_results]
        successful = sum(1 for r in results if r["success"])
        latencies = [r["latency_ms"] for r in results if r["success"]]
        avg_latency = sum(latencies) / len(latencies) if latencies else 0
        success_rate = successful / len(results) * 100
        
        status = "🟢 Healthy" if success_rate > 66 else "🟡 Degraded" if success_rate > 33 else "🔴 Down"
        print(f"{system_name:<12} {avg_latency:<15.0f} {success_rate:<15.0f}% {status}")
    
    print("\n" + "="*60)
    print("BENCHMARK COMPLETE")
    print("="*60)
    
    return {
        "timestamp": datetime.now().isoformat(),
        "projects_indexed": len(project_index),
        "benchmark_results": benchmark_results
    }

if __name__ == "__main__":
    result = asyncio.run(main())
    
    # Save results
    output_file = f"E:\\scripts-python\\xavier2\\benchmark_results\\benchmark_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    Path(output_file).parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\n💾 Results saved to: {output_file}")
