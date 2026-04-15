#!/usr/bin/env python3
"""
Real memory benchmark using OpenClaw Xavier2 operations.
Tests the ACTUAL behavior of Xavier2 with real HTTP API calls.
No hardcoded expectations - measures real recall, latency, and throughput.
"""

import json
import time
import urllib.request
import urllib.error
from datetime import datetime

BASE_URL = "http://localhost:8003"
TOKEN = "dev-token"
OUTPUT_DIR = "benchmark-results/real-memory-benchmark"


def api(path: str, payload: dict = None, method: str = "POST") -> dict:
    url = f"{BASE_URL}{path}"
    data = json.dumps(payload).encode("utf-8") if payload else None
    req = urllib.request.Request(
        url, data=data, method=method,
        headers={"Content-Type": "application/json", "X-Xavier2-Token": TOKEN}
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        if e.code == 405 and method == "GET":
            req = urllib.request.Request(url, method="GET",
                headers={"X-Xavier2-Token": TOKEN})
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.loads(r.read().decode("utf-8"))
        raise


def wait_health():
    for _ in range(60):
        try:
            with urllib.request.urlopen(f"{BASE_URL}/health", timeout=5) as r:
                if r.status == 200:
                    return
        except Exception:
            time.sleep(1)
    raise RuntimeError("Xavier2 not healthy")


def reset_and_load(documents: list) -> float:
    api("/memory/reset", {})
    elapsed = time.time()
    for doc in documents:
        api("/memory/add", {
            "path": doc["path"],
            "content": doc["content"],
            "metadata": doc.get("metadata", {}),
            "kind": doc.get("kind"),
            "evidence_kind": doc.get("evidence_kind"),
            "namespace": doc.get("namespace"),
            "provenance": doc.get("provenance"),
        })
    return (time.time() - elapsed) / len(documents) * 1000


def search(query: str, filters: dict = None, limit: int = 5):
    payload = {"query": query, "limit": limit}
    if filters:
        payload["filters"] = filters
    t0 = time.time()
    resp = api("/memory/search", payload)
    return resp.get("results", []), (time.time() - t0) * 1000


def query_endpoint(query: str, filters: dict = None, system3: str = "disabled"):
    payload = {"query": query, "limit": 5}
    if filters:
        payload["filters"] = filters
    payload["system3_mode"] = system3
    t0 = time.time()
    resp = api("/memory/query", payload)
    return resp.get("response", ""), (time.time() - t0) * 1000


def run_agents(query: str, filters: dict = None):
    payload = {"query": query, "limit": 5}
    if filters:
        payload["filters"] = filters
    t0 = time.time()
    resp = api("/agents/run", payload)
    return resp.get("response", ""), (time.time() - t0) * 1000


def run_benchmark():
    import os
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print(f"Waiting for Xavier2 at {BASE_URL}...")
    wait_health()
    print("Xavier2 is healthy.")

    dataset_path = "E:\\scripts-python\\xavier2\\scripts\\benchmarks\\datasets\\internal_swal_openclaw_memory.json"
    with open(dataset_path, encoding="utf-8") as f:
        dataset = json.load(f)

    documents = dataset["documents"]
    cases = dataset["cases"]

    print(f"\nLoading {len(documents)} documents via OpenClaw Xavier2 API...")
    load_time_ms = reset_and_load(documents)
    print(f"  [OK] Load time: {load_time_ms:.1f}ms per document")

    results = []
    search_latencies = []
    query_latencies = []
    correct = 0
    total = len(cases)

    print(f"\nRunning {total} benchmark cases via OpenClaw Xavier2 API...")
    for case in cases:
        cid = case["id"]
        endpoint = case["endpoint"]
        query = case["query"]
        filters = case.get("filters")

        if endpoint == "search":
            results_list, lat_ms = search(query, filters)
            search_latencies.append(lat_ms)
            top_path = results_list[0].get("path") if results_list else None
            expected = case.get("expected_path")
            hit = top_path == expected
            results.append({
                "id": cid,
                "endpoint": endpoint,
                "query": query,
                "expected": expected,
                "actual": top_path,
                "hit": hit,
                "latency_ms": round(lat_ms, 1),
                "results_count": len(results_list),
            })
        elif endpoint == "query":
            response, lat_ms = query_endpoint(query, filters, case.get("system3_mode", "disabled"))
            query_latencies.append(lat_ms)
            expected_sub = case.get("expected_substring", "")
            hit = expected_sub.lower() in response.lower()
            results.append({
                "id": cid,
                "endpoint": endpoint,
                "query": query,
                "expected_substring": expected_sub,
                "actual_response": response[:200],
                "hit": hit,
                "latency_ms": round(lat_ms, 1),
            })
        elif endpoint == "agents_run":
            response, lat_ms = run_agents(query, filters)
            query_latencies.append(lat_ms)
            expected_sub = case.get("expected_substring", "")
            hit = expected_sub.lower() in response.lower()
            results.append({
                "id": cid,
                "endpoint": endpoint,
                "query": query,
                "expected_substring": expected_sub,
                "actual_response": response[:200],
                "hit": hit,
                "latency_ms": round(lat_ms, 1),
            })

        correct += 1 if results[-1]["hit"] else 0
        status = "[PASS]" if results[-1]["hit"] else "[FAIL]"
        print(f"  {status} {cid} ({lat_ms:.0f}ms)")

    accuracy = correct / total * 100
    avg_search_lat = sum(search_latencies) / len(search_latencies) if search_latencies else 0
    avg_query_lat = sum(query_latencies) / len(query_latencies) if query_latencies else 0

    build = api("/build", {}, "GET")
    summary = {
        "timestamp": datetime.now().isoformat(),
        "backend": build["memory_store"]["backend"],
        "version": build["version"],
        "total_cases": total,
        "passed": correct,
        "accuracy_pct": round(accuracy, 1),
        "avg_search_latency_ms": round(avg_search_lat, 1),
        "avg_query_latency_ms": round(avg_query_lat, 1),
        "load_time_ms_per_doc": round(load_time_ms, 1),
        "search_cases": len(search_latencies),
        "query_cases": len(query_latencies),
    }

    print(f"\n{'='*50}")
    print(f"RESULTS -- {summary['backend']} backend (v{summary['version']})")
    print(f"{'='*50}")
    print(f"Accuracy:     {accuracy:.1f}%  ({correct}/{total})")
    print(f"Avg search:   {avg_search_lat:.1f}ms")
    print(f"Avg query:    {avg_query_lat:.1f}ms")
    print(f"Load/doc:     {load_time_ms:.1f}ms")

    summary_path = f"{OUTPUT_DIR}/summary.json"
    records_path = f"{OUTPUT_DIR}/records.json"
    with open(summary_path, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)
    with open(records_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2)

    print(f"\nSaved: {summary_path}")
    print(f"Saved: {records_path}")
    return summary


if __name__ == "__main__":
    run_benchmark()