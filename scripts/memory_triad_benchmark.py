#!/usr/bin/env python3
"""
SWAL memory triad benchmark.

Compares the three memory systems with their most reliable local interface:
- Cortex: HTTP API on 127.0.0.1:8003
- Xavier2: HTTP API on 127.0.0.1:8006
- Engram: local CLI binary

The script can optionally start Xavier2 from the release binary and writes a JSON
report under E:/scripts-python/xavier2/benchmark_results.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RESULTS_DIR = ROOT / "benchmark_results"

DEFAULT_CORTEX_URL = "http://127.0.0.1:8003"
DEFAULT_XAVIER2_URL = "http://127.0.0.1:8006"
DEFAULT_ENGRAM_BIN = r"C:\Users\belal\AppData\Local\Temp\engram\engram.exe"
GLOBAL_XAVIER2_BIN = Path(r"C:\Users\belal\.cargo\target_global\release\xavier2.exe")
DEFAULT_XAVIER2_BIN = str(
    GLOBAL_XAVIER2_BIN if GLOBAL_XAVIER2_BIN.is_file() else ROOT / "target" / "release" / "xavier2.exe"
)

TOKEN = "dev-token"


@dataclass
class OperationResult:
    ok: bool
    latency_ms: float
    payload: Any = None
    error: str | None = None

    def to_json(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "latency_ms": round(self.latency_ms, 2),
            "payload": self.payload,
            "error": self.error,
        }


TEST_MEMORIES = [
    {
        "id": "model-default",
        "content": "SWAL default model decision: use MiniMax-M2.7 for routine agents unless a task requires a different provider.",
        "query": "What is SWAL's default model?",
        "expected": "MiniMax",
    },
    {
        "id": "memory-architecture",
        "content": "OpenClaw memory architecture: Xavier2 is the fast local/vector memory, Cortex is the institutional memory, and Engram is the external baseline.",
        "query": "Which three memory systems are being compared?",
        "expected": "Xavier2",
    },
    {
        "id": "xavier-port",
        "content": "Operational ports: Cortex listens on 8003, Xavier2 should listen on 8006, and Engram is used through its local CLI binary.",
        "query": "Which port should Xavier2 use?",
        "expected": "8006",
    },
]

DEFAULT_XAVIER2_CODE_CONTEXT_PATH = ROOT
DEFAULT_CORTEX_CODE_CONTEXT_PATH = os.environ.get(
    "CORTEX_CODE_CONTEXT_PATH",
    "/mnt/workspaces/xavier2",
)
CODE_CONTEXT_QUERIES = [
    {
        "query": "SemanticCache",
        "expected": "SemanticCache",
        "kind": "struct",
    },
    {
        "query": "CodeGraphDB",
        "expected": "CodeGraphDB",
        "kind": "struct",
    },
    {
        "query": "code_scan_handler",
        "expected": "code_scan_handler",
        "kind": "function",
    },
    {
        "query": "memory_add",
        "expected": "memory_add",
        "kind": "function",
    },
    {
        "query": "QmdMemory",
        "expected": "QmdMemory",
        "kind": "struct",
    },
]


def now_stamp() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S_%f")


def build_run_memories(run_id: str) -> list[dict[str, str]]:
    """Attach a run marker so persisted old memories cannot count as hits."""
    marker = f"benchmark_run_id:{run_id}"
    memories = []
    for item in TEST_MEMORIES:
        memories.append(
            {
                **item,
                "run_id": run_id,
                "marker": marker,
                "content": f"{item['content']} Marker for this benchmark only: {marker}.",
                "query": item["query"],
            }
        )
    return memories


def elapsed_ms(start: float) -> float:
    return (time.perf_counter() - start) * 1000


def http_json(
    base_url: str,
    path: str,
    payload: dict[str, Any] | None = None,
    timeout: float = 8.0,
) -> OperationResult:
    start = time.perf_counter()
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    method = "GET" if payload is None else "POST"
    req = urllib.request.Request(
        f"{base_url.rstrip('/')}{path}",
        data=data,
        method=method,
        headers={
            "Content-Type": "application/json",
            "X-Cortex-Token": TOKEN,
            "X-Xavier2-Token": TOKEN,
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            body = response.read().decode("utf-8", errors="replace")
            parsed = json.loads(body) if body else {}
            return OperationResult(
                ok=200 <= response.status < 300,
                latency_ms=elapsed_ms(start),
                payload=parsed,
            )
    except urllib.error.HTTPError as error:
        try:
            body = error.read().decode("utf-8", errors="replace")
            parsed = json.loads(body) if body else {"error": body}
        except Exception:
            parsed = {"error": str(error)}
        return OperationResult(
            False,
            elapsed_ms(start),
            payload=parsed,
            error=f"HTTP {error.code}: {error.reason}",
        )
    except Exception as error:
        return OperationResult(False, elapsed_ms(start), error=str(error))


def wait_for_http(base_url: str, timeout_seconds: int = 20) -> bool:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        if http_json(base_url, "/health", timeout=2.0).ok:
            return True
        time.sleep(0.5)
    return False


def start_xavier2_if_needed(xavier2_url: str, xavier2_bin: str) -> dict[str, Any]:
    if http_json(xavier2_url, "/health", timeout=2.0).ok:
        return {"started": False, "reason": "already_running"}

    binary = Path(xavier2_bin)
    if not binary.is_file():
        return {"started": False, "error": f"missing binary: {binary}"}

    logs = ROOT / "benchmark_results"
    logs.mkdir(parents=True, exist_ok=True)
    stdout_path = logs / "xavier2-triad-8006.out.log"
    stderr_path = logs / "xavier2-triad-8006.err.log"

    stdout = stdout_path.open("ab")
    stderr = stderr_path.open("ab")
    subprocess.Popen(
        [str(binary), "http", "8006"],
        cwd=str(ROOT),
        stdout=stdout,
        stderr=stderr,
        creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
    )

    ready = wait_for_http(xavier2_url, timeout_seconds=25)
    return {
        "started": ready,
        "reason": "launched",
        "stdout": str(stdout_path),
        "stderr": str(stderr_path),
    }


def engram(command: list[str], engram_bin: str, timeout: int = 10) -> OperationResult:
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            [engram_bin, *command],
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(ROOT),
        )
        payload = {
            "returncode": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
        }
        return OperationResult(proc.returncode == 0, elapsed_ms(start), payload=payload)
    except Exception as error:
        return OperationResult(False, elapsed_ms(start), error=str(error))


def contains_expected(payload: Any, expected: str, marker: str | None = None) -> bool:
    if isinstance(payload, dict):
        for key in ("results", "content", "symbols"):
            values = payload.get(key)
            if isinstance(values, list):
                return any(contains_expected(item, expected, marker) for item in values)

    text = json.dumps(payload, ensure_ascii=False).lower()
    if expected.lower() not in text:
        return False
    return marker is None or marker.lower() in text


def hit_count_from_http(payload: Any) -> int:
    if not isinstance(payload, dict):
        return 0
    if isinstance(payload.get("results"), list):
        return len(payload["results"])
    if isinstance(payload.get("content"), list):
        return len(payload["content"])
    count = payload.get("count")
    return int(count) if isinstance(count, int) else 0


def extract_symbol_hits(payload: Any) -> int:
    if not isinstance(payload, dict):
        return 0
    if isinstance(payload.get("results"), list):
        return len(payload["results"])
    if isinstance(payload.get("symbols"), list):
        return len(payload["symbols"])
    return 0


def run_http_system(
    name: str,
    base_url: str,
    memories: list[dict[str, str]],
    run_id: str,
) -> dict[str, Any]:
    health = http_json(base_url, "/health", timeout=3.0)
    result: dict[str, Any] = {
        "type": "http",
        "url": base_url,
        "health": health.to_json(),
        "writes": [],
        "searches": [],
    }
    if not health.ok:
        return result

    for item in memories:
        write = http_json(
            base_url,
            "/memory/add",
            {
                "path": f"triad/{run_id}/{item['id']}",
                "content": item["content"],
                "metadata": {
                    "benchmark": "memory_triad",
                    "source": "memory_triad_benchmark",
                    "system_under_test": name,
                    "run_id": run_id,
                    "marker": item["marker"],
                    "project": run_id,
                    "scope": "benchmark",
                },
            },
        )
        result["writes"].append(write.to_json())

    time.sleep(0.2)

    for item in memories:
        search = http_json(
            base_url,
            "/memory/search",
            {
                "query": item["query"],
                "limit": 5,
                "filters": {
                    "project": run_id,
                    "scope": "benchmark",
                },
            },
        )
        result["searches"].append(
            {
                **search.to_json(),
                "query": item["query"],
                "expected": item["expected"],
                "run_id": run_id,
                "marker": item["marker"],
                "matched_expected": contains_expected(
                    search.payload,
                    item["expected"],
                    item["marker"],
                ),
                "hit_count": hit_count_from_http(search.payload),
            }
        )

    return result


def run_engram(engram_bin: str, memories: list[dict[str, str]], run_id: str) -> dict[str, Any]:
    binary_exists = Path(engram_bin).is_file()
    result: dict[str, Any] = {
        "type": "cli",
        "binary": engram_bin,
        "health": {"ok": binary_exists, "error": None if binary_exists else "binary missing"},
        "writes": [],
        "searches": [],
    }
    if not binary_exists:
        return result

    version = engram(["version"], engram_bin)
    result["version"] = version.to_json()
    project = f"swal-memory-triad-{run_id}"

    for item in memories:
        write = engram(
            [
                "save",
                f"triad/{run_id}/{item['id']}",
                item["content"],
                "--project",
                project,
                "--scope",
                "benchmark",
            ],
            engram_bin,
        )
        result["writes"].append(write.to_json())

    time.sleep(0.2)

    for item in memories:
        search = engram(
            [
                "search",
                item["query"],
                "--project",
                project,
                "--limit",
                "5",
            ],
            engram_bin,
        )
        stdout = ""
        if isinstance(search.payload, dict):
            stdout = search.payload.get("stdout", "") or ""
        has_no_results = stdout.strip().lower().startswith("no memories found")
        result["searches"].append(
            {
                **search.to_json(),
                "query": item["query"],
                "expected": item["expected"],
                "run_id": run_id,
                "marker": item["marker"],
                "matched_expected": item["expected"].lower() in stdout.lower()
                and item["marker"].lower() in stdout.lower(),
                "hit_count": 0
                if has_no_results
                else len([line for line in stdout.splitlines() if line.strip()]),
            }
        )

    return result


def run_code_context_http_system(name: str, base_url: str, code_path: str) -> dict[str, Any]:
    health = http_json(base_url, "/health", timeout=3.0)
    result: dict[str, Any] = {
        "type": "http-code-context",
        "url": base_url,
        "health": health.to_json(),
        "scan": None,
        "find": [],
    }
    if not health.ok:
        return result

    scan = http_json(
        base_url,
        "/code/scan",
        {"path": code_path},
        timeout=120.0,
    )
    result["scan"] = {
        **scan.to_json(),
        "path": code_path,
        "indexed_files": (scan.payload or {}).get("indexed_files") if isinstance(scan.payload, dict) else None,
        "indexed_symbols": (
            (scan.payload or {}).get("indexed_symbols")
            or (scan.payload or {}).get("indexed_chunks")
        )
        if isinstance(scan.payload, dict)
        else None,
        "indexed_imports": (scan.payload or {}).get("indexed_imports") if isinstance(scan.payload, dict) else None,
    }

    for item in CODE_CONTEXT_QUERIES:
        find = http_json(
            base_url,
            "/code/find",
            {
                "query": item["query"],
                "limit": 5,
                "kind": item["kind"],
            },
            timeout=30.0,
        )
        result["find"].append(
            {
                **find.to_json(),
                "query": item["query"],
                "expected": item["expected"],
                "matched_expected": contains_expected(find.payload, item["expected"]),
                "hit_count": extract_symbol_hits(find.payload),
            }
        )

    return result


def run_code_context_engram(engram_bin: str, code_path: str) -> dict[str, Any]:
    binary_exists = Path(engram_bin).is_file()
    result: dict[str, Any] = {
        "type": "cli-code-context",
        "binary": engram_bin,
        "health": {"ok": binary_exists, "error": None if binary_exists else "binary missing"},
        "scan": {"available": False, "reason": "engram_has_no_code_scan_endpoint"},
        "not_comparable": True,
        "reason": "Engram is memory/session retrieval, not a source-code scanner.",
        "find": [],
    }
    return result


def summarize(system_result: dict[str, Any]) -> dict[str, Any]:
    searches = system_result.get("searches") or []
    writes = system_result.get("writes") or []
    search_ok = [s for s in searches if s.get("ok")]
    write_ok = [w for w in writes if w.get("ok")]
    matched = [s for s in searches if s.get("matched_expected")]
    latencies = [float(s["latency_ms"]) for s in search_ok if "latency_ms" in s]
    return {
        "available": bool(system_result.get("health", {}).get("ok")),
        "writes_ok": len(write_ok),
        "writes_total": len(writes),
        "searches_ok": len(search_ok),
        "searches_total": len(searches),
        "matched_expected": len(matched),
        "avg_search_latency_ms": round(sum(latencies) / len(latencies), 2) if latencies else None,
    }


def summarize_code_context(system_result: dict[str, Any]) -> dict[str, Any]:
    if not system_result.get("health", {}).get("ok"):
        return {"available": False}
    if system_result.get("type") == "http-code-context":
        find = system_result.get("find") or []
        matched = [item for item in find if item.get("matched_expected")]
        latencies = [float(item["latency_ms"]) for item in find if item.get("ok")]
        scan = system_result.get("scan") or {}
        return {
            "available": True,
            "scan_ok": bool(scan.get("ok")),
            "indexed_files": scan.get("indexed_files"),
            "indexed_symbols": scan.get("indexed_symbols"),
            "matched_expected": len(matched),
            "queries_total": len(find),
            "avg_find_latency_ms": round(sum(latencies) / len(latencies), 2) if latencies else None,
        }
    find = system_result.get("find") or []
    if system_result.get("not_comparable"):
        return {
            "available": True,
            "scan_ok": False,
            "indexed_files": None,
            "indexed_symbols": None,
            "matched_expected": 0,
            "queries_total": 0,
            "avg_find_latency_ms": None,
            "not_comparable": True,
        }
    matched = [item for item in find if item.get("matched_expected")]
    latencies = [float(item["latency_ms"]) for item in find if item.get("ok")]
    return {
        "available": True,
        "scan_ok": False,
        "indexed_files": None,
        "indexed_symbols": None,
        "matched_expected": len(matched),
        "queries_total": len(find),
        "avg_find_latency_ms": round(sum(latencies) / len(latencies), 2) if latencies else None,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cortex-url", default=DEFAULT_CORTEX_URL)
    parser.add_argument("--xavier2-url", default=DEFAULT_XAVIER2_URL)
    parser.add_argument("--engram-bin", default=os.environ.get("ENGRAM_BIN", DEFAULT_ENGRAM_BIN))
    parser.add_argument("--xavier2-bin", default=os.environ.get("XAVIER2_BIN", DEFAULT_XAVIER2_BIN))
    parser.add_argument(
        "--cortex-code-path",
        default=DEFAULT_CORTEX_CODE_CONTEXT_PATH,
        help="Path visible from the Cortex process/container for code scan",
    )
    parser.add_argument(
        "--xavier2-code-path",
        default=str(DEFAULT_XAVIER2_CODE_CONTEXT_PATH),
        help="Path visible from the Xavier2 process for code scan",
    )
    parser.add_argument("--start-xavier2", action="store_true")
    parser.add_argument("--no-write", action="store_true")
    args = parser.parse_args()

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    startup = None
    if args.start_xavier2:
        startup = start_xavier2_if_needed(args.xavier2_url, args.xavier2_bin)

    run_id = f"swal-triad-{now_stamp()}"
    memories = [] if args.no_write else build_run_memories(run_id)
    report = {
        "timestamp": datetime.now().isoformat(),
        "run_id": run_id,
        "goal": "Compare Xavier2, Cortex, and Engram using stable local interfaces",
        "startup": {"xavier2": startup},
        "systems": {
            "cortex": run_http_system("cortex", args.cortex_url, memories, run_id),
            "xavier2": run_http_system("xavier2", args.xavier2_url, memories, run_id),
            "engram": run_engram(args.engram_bin, memories, run_id),
        },
        "code_context": {
            "target_paths": {
                "cortex": args.cortex_code_path,
                "xavier2": args.xavier2_code_path,
                "engram": args.xavier2_code_path,
            },
            "systems": {
                "cortex": run_code_context_http_system(
                    "cortex",
                    args.cortex_url,
                    args.cortex_code_path,
                ),
                "xavier2": run_code_context_http_system(
                    "xavier2",
                    args.xavier2_url,
                    args.xavier2_code_path,
                ),
                "engram": run_code_context_engram(args.engram_bin, args.xavier2_code_path),
            },
        },
    }
    report["summary"] = {
        name: summarize(result) for name, result in report["systems"].items()
    }
    report["code_context"]["summary"] = {
        name: summarize_code_context(result)
        for name, result in report["code_context"]["systems"].items()
    }

    output_path = RESULTS_DIR / f"memory_triad_{now_stamp()}.json"
    output_path.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")

    print(
        json.dumps(
            {
                "report": str(output_path),
                "summary": report["summary"],
                "code_context_summary": report["code_context"]["summary"],
            },
            indent=2,
        )
    )

    # A low recall score is benchmark data, not a runner failure. Non-zero is
    # reserved for infrastructure being unavailable.
    all_available = all(item["available"] for item in report["summary"].values())
    xavier_code = report["code_context"]["summary"].get("xavier2", {})
    xavier_code_ok = xavier_code.get("scan_ok") and xavier_code.get("matched_expected", 0) >= 1
    return 0 if all_available and xavier_code_ok else 2


if __name__ == "__main__":
    sys.exit(main())
