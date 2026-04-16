"""
SWAL Memory Benchmark: Xavier2 vs Cortex vs Engram
===================================================
Run both memory systems in parallel, send same queries, compare results.

Usage:
    python benchmark_memory_systems.py
"""

import asyncio
import json
import time
import statistics
from datetime import datetime
from typing import List, Dict, Any

# Configuration
SYSTEMS = {
    "cortex": {
        "url": "http://localhost:8003", 
        "endpoints": {
            "add": "/memory/add",
            "search": "/memory/search",
            "retrieve": "/memory/retrieve",
            "stats": "/stats",
        }
    },
    "xavier2": {
        "url": "http://localhost:8006",
        "endpoints": {
            "add": "/memory/add",
            "search": "/memory/search",
            "retrieve": "/memory/retrieve",
            "stats": "/stats",
        }
    },
    "engram": {
        "url": "http://localhost:7437",
        "endpoints": {
            "add": "/prompts",
            "search": "/mem/search",
            "retrieve": "/mem/search",
            "stats": "/stats",
        }
    }
}

# Test scenarios
SCENARIOS = [
    {
        "name": "fact_storage",
        "query": "Remember that SWAL uses MiniMax-M2.7 as default model",
        "expected_context": "model preference",
    },
    {
        "name": "client_info", 
        "query": "Store: Leonardo Duque is a partner working on Rodacenter Chile",
        "expected_context": "Leonardo, Rodacenter, Chile",
    },
    {
        "name": "decision_log",
        "query": "Decision: Xavier2 Docker uses multi-stage build for minimal image",
        "expected_context": "Docker, multi-stage build",
    },
    {
        "name": "code_recall",
        "query": "Remember: xavier2 API uses port 8006, cortex uses 8003",
        "expected_context": "ports, xavier2, cortex",
    },
    {
        "name": "multi_hop",
        "query": "Find the decision about which model to use for SWAL agents",
        "expected_context": "MiniMax, decision, agents",
    },
]

class MemoryBenchmark:
    def __init__(self):
        self.results = {}
        self.latencies = {name: [] for name in SYSTEMS}
        self.accuracy = {name: [] for name in SYSTEMS}
    
    async def send_to_system(self, system_name: str, query: str) -> Dict[str, Any]:
        """Send a query to a memory system and measure latency."""
        import aiohttp
        
        config = SYSTEMS[system_name]
        url = config["url"] + config["endpoints"]["add"]
        
        start = time.time()
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    url,
                    json={"content": query, "timestamp": datetime.now().isoformat()},
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    latency = (time.time() - start) * 1000  # ms
                    result = await resp.json()
                    return {
                        "success": resp.status == 200,
                        "latency_ms": latency,
                        "result": result
                    }
        except Exception as e:
            return {
                "success": False,
                "latency_ms": (time.time() - start) * 1000,
                "error": str(e)
            }
    
    async def query_system(self, system_name: str, query: str) -> Dict[str, Any]:
        """Query a memory system and measure latency."""
        import aiohttp
        
        config = SYSTEMS[system_name]
        url = config["url"] + config["endpoints"]["search"]
        
        start = time.time()
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    url,
                    json={"query": query},
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    latency = (time.time() - start) * 1000
                    result = await resp.json()
                    return {
                        "success": resp.status == 200,
                        "latency_ms": latency,
                        "result": result
                    }
        except Exception as e:
            return {
                "success": False,
                "latency_ms": (time.time() - start) * 1000,
                "error": str(e)
            }
    
    async def run_scenario(self, scenario: Dict) -> Dict[str, Any]:
        """Run a single scenario across all systems."""
        print(f"\n📋 Scenario: {scenario['name']}")
        print(f"   Query: {scenario['query'][:60]}...")
        
        results = {}
        
        # Store in all systems
        for system_name in SYSTEMS:
            result = await self.send_to_system(system_name, scenario["query"])
            results[system_name] = result
            self.latencies[system_name].append(result["latency_ms"])
            
            if result["success"]:
                print(f"   ✅ {system_name}: {result['latency_ms']:.1f}ms")
            else:
                print(f"   ❌ {system_name}: {result.get('error', 'Unknown error')}")
        
        # Small delay
        await asyncio.sleep(0.5)
        
        # Query all systems
        query_results = {}
        for system_name in SYSTEMS:
            result = await self.query_system(system_name, scenario["query"])
            query_results[system_name] = result
            self.latencies[system_name].append(result["latency_ms"])
            
            if result["success"]:
                print(f"   🔍 {system_name} recall: {len(result.get('result', {}).get('matches', []))} matches")
            else:
                print(f"   ❌ {system_name} recall failed")
        
        return {
            "scenario": scenario["name"],
            "store_results": results,
            "query_results": query_results
        }
    
    async def run_benchmark(self):
        """Run full benchmark across all systems and scenarios."""
        print("=" * 60)
        print("SWAL MEMORY BENCHMARK: Xavier2 vs Cortex vs Engram")
        print("=" * 60)
        print(f"Systems: {list(SYSTEMS.keys())}")
        print(f"Scenarios: {len(SCENARIOS)}")
        print()
        
        start_time = time.time()
        
        # Run all scenarios
        for scenario in SCENARIOS:
            await self.run_scenario(scenario)
        
        total_time = time.time() - start_time
        
        # Generate report
        print("\n" + "=" * 60)
        print("BENCHMARK RESULTS")
        print("=" * 60)
        
        for system_name in SYSTEMS:
            latencies = self.latencies[system_name]
            if latencies:
                avg = statistics.mean(latencies)
                median = statistics.median(latencies)
                p95 = sorted(latencies)[int(len(latencies) * 0.95)] if len(latencies) > 1 else avg
                success_rate = sum(1 for r in latencies if r > 0) / len(latencies)
                
                print(f"\n📊 {system_name.upper()}")
                print(f"   Avg latency: {avg:.1f}ms")
                print(f"   Median: {median:.1f}ms")
                print(f"   P95: {p95:.1f}ms")
                print(f"   Operations: {len(latencies)}")
        
        print(f"\n⏱️  Total time: {total_time:.1f}s")
        
        # Save results
        report = {
            "timestamp": datetime.now().isoformat(),
            "systems": list(SYSTEMS.keys()),
            "scenarios": len(SCENARIOS),
            "latencies": self.latencies,
            "total_time_seconds": total_time
        }
        
        output_file = f"benchmark_results_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(output_file, "w") as f:
            json.dump(report, f, indent=2)
        
        print(f"\n💾 Results saved to: {output_file}")
        
        return report

if __name__ == "__main__":
    benchmark = MemoryBenchmark()
    asyncio.run(benchmark.run_benchmark())
