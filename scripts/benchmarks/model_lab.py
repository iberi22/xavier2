#!/usr/bin/env python3
import argparse
import hashlib
import json
import math
import os
import socket
import statistics
import subprocess
import time
import urllib.parse
import urllib.request
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DATASET = (
    ROOT / "scripts" / "benchmarks" / "datasets" / "internal_swal_openclaw_memory.json"
)
DEFAULT_BASE_URL = "http://127.0.0.1:8003"
DEFAULT_TOKEN = os.environ.get("XAVIER2_TOKEN", "dev-token")
HTTP_TIMEOUT_SECONDS = 90


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: str | Path) -> dict[str, Any]:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def http_json(url: str, payload: dict | None = None, method: str = "GET") -> dict[str, Any]:
    headers = {
        "Content-Type": "application/json",
        "X-Xavier2-Token": DEFAULT_TOKEN,
    }
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url, data=data, method=method, headers=headers)
    with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(base_url: str) -> None:
    for _ in range(60):
        try:
            with urllib.request.urlopen(f"{base_url}/health", timeout=5) as response:
                payload = json.loads(response.read().decode("utf-8"))
                if response.status == 200 and payload.get("status") == "ok":
                    return
        except Exception:
            time.sleep(1)
    raise RuntimeError("Xavier2 did not become healthy in time")


def reserve_base_url(base_url: str) -> str:
    parsed = urllib.parse.urlparse(base_url)
    host = parsed.hostname or "127.0.0.1"
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((host, 0))
        port = sock.getsockname()[1]
    return urllib.parse.urlunparse(
        (
            parsed.scheme or "http",
            f"{host}:{port}",
            parsed.path or "",
            "",
            "",
            "",
        )
    )


def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    if len(values) == 1:
        return values[0]
    sorted_values = sorted(values)
    rank = (len(sorted_values) - 1) * pct
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return sorted_values[int(rank)]
    lower_value = sorted_values[lower]
    upper_value = sorted_values[upper]
    return lower_value + (upper_value - lower_value) * (rank - lower)


def launch_xavier2(
    base_url: str,
    output_dir: Path,
    env_overrides: dict[str, str],
) -> subprocess.Popen[bytes]:
    parsed = urllib.parse.urlparse(base_url)
    env = os.environ.copy()
    env.setdefault("XAVIER2_TOKEN", DEFAULT_TOKEN)
    env["XAVIER2_HOST"] = parsed.hostname or "127.0.0.1"
    if parsed.port:
        env["XAVIER2_PORT"] = str(parsed.port)
    env.update({key: value for key, value in env_overrides.items() if value is not None})

    stdout_path = output_dir / "xavier2.stdout.log"
    stderr_path = output_dir / "xavier2.stderr.log"
    output_dir.mkdir(parents=True, exist_ok=True)
    return subprocess.Popen(
        ["cargo", "run", "--bin", "xavier2"],
        cwd=ROOT,
        env=env,
        stdout=stdout_path.open("wb"),
        stderr=stderr_path.open("wb"),
    )


def stop_process(child: subprocess.Popen[bytes] | None) -> None:
    if child is None:
        return
    child.terminate()
    try:
        child.wait(timeout=15)
    except subprocess.TimeoutExpired:
        child.kill()


def reset_and_seed_memory(base_url: str, dataset: dict[str, Any]) -> dict[str, dict[str, Any]]:
    http_json(f"{base_url}/memory/reset", {}, method="POST")
    documents_by_path: dict[str, dict[str, Any]] = {}
    for document in dataset.get("documents", []):
        payload = {
            "path": document["path"],
            "content": document["content"],
            "metadata": document.get("metadata", {}),
            "kind": document.get("kind"),
            "evidence_kind": document.get("evidence_kind"),
            "namespace": document.get("namespace"),
            "provenance": document.get("provenance"),
        }
        http_json(f"{base_url}/memory/add", payload, method="POST")
        documents_by_path[document["path"]] = document
    return documents_by_path


def evaluate_search_case(base_url: str, case: dict[str, Any]) -> dict[str, Any]:
    payload = {
        "query": case["query"],
        "limit": 10,
        "filters": case.get("filters"),
    }
    started = time.perf_counter()
    response = http_json(f"{base_url}/memory/search", payload, method="POST")
    latency_ms = (time.perf_counter() - started) * 1000.0
    ranked_paths = [item.get("path") for item in response.get("results", [])]
    expected_path = case["expected_path"]
    rank = ranked_paths.index(expected_path) + 1 if expected_path in ranked_paths else None
    return {
        "id": case["id"],
        "kind": "search",
        "success": rank is not None,
        "expected_path": expected_path,
        "rank": rank,
        "results": ranked_paths,
        "latency_ms": latency_ms,
    }


def evaluate_generation_case(
    base_url: str,
    case: dict[str, Any],
    force_system3_mode: str | None,
) -> dict[str, Any]:
    payload = {
        "query": case["query"],
        "filters": case.get("filters"),
    }
    if force_system3_mode:
        payload["system3_mode"] = force_system3_mode
    elif case.get("system3_mode"):
        payload["system3_mode"] = case["system3_mode"]

    route = "/memory/query" if case["endpoint"] == "query" else "/agents/run"
    started = time.perf_counter()
    response = http_json(f"{base_url}{route}", payload, method="POST")
    latency_ms = (time.perf_counter() - started) * 1000.0
    answer = response.get("response", "")
    expected = case["expected_substring"]
    success = expected.lower() in answer.lower()
    citation_like = any(
        needle in answer.lower()
        for needle in ["source", "memory/", "repo/", "session/", "decision/"]
    )
    return {
        "id": case["id"],
        "kind": "generation",
        "success": success,
        "expected_substring": expected,
        "actual_response": answer,
        "citation_like": citation_like,
        "latency_ms": latency_ms,
        "endpoint": case["endpoint"],
    }


def compute_search_metrics(
    records: list[dict[str, Any]],
    documents_by_path: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    if not records:
        return {}

    recall_at_5 = []
    recall_at_10 = []
    mrr = []
    ndcg_at_10 = []
    high_priority_hits = []
    by_priority: dict[str, list[int]] = defaultdict(list)

    for record in records:
        rank = record.get("rank")
        priority = (
            documents_by_path.get(record["expected_path"], {})
            .get("metadata", {})
            .get("memory_priority", "medium")
        )
        recall_at_5.append(int(rank is not None and rank <= 5))
        recall_at_10.append(int(rank is not None and rank <= 10))
        mrr.append(0.0 if rank is None else 1.0 / rank)
        ndcg_at_10.append(0.0 if rank is None or rank > 10 else 1.0 / math.log2(rank + 1))
        by_priority[priority].append(int(rank is not None and rank <= 10))
        if priority in {"critical", "high"}:
            high_priority_hits.append(int(rank is not None and rank <= 10))

    return {
        "cases": len(records),
        "recall_at_5": sum(recall_at_5) / len(recall_at_5),
        "recall_at_10": sum(recall_at_10) / len(recall_at_10),
        "mrr": sum(mrr) / len(mrr),
        "ndcg_at_10": sum(ndcg_at_10) / len(ndcg_at_10),
        "high_priority_recall_at_10": (
            sum(high_priority_hits) / len(high_priority_hits) if high_priority_hits else None
        ),
        "priority_breakdown": {
            priority: sum(values) / len(values) for priority, values in sorted(by_priority.items())
        },
    }


def compute_generation_metrics(records: list[dict[str, Any]]) -> dict[str, Any]:
    if not records:
        return {}

    matches = [int(record["success"]) for record in records]
    citations = [int(record["citation_like"]) for record in records]
    return {
        "cases": len(records),
        "faithfulness_score": sum(matches) / len(matches),
        "citation_score": sum(citations) / len(citations),
        "match_rate": sum(matches) / len(matches),
    }


def summarize_latencies(records: list[dict[str, Any]]) -> dict[str, float]:
    latencies = [record["latency_ms"] for record in records]
    if not latencies:
        return {"latency_p50_ms": 0.0, "latency_p95_ms": 0.0, "latency_avg_ms": 0.0}
    return {
        "latency_p50_ms": percentile(latencies, 0.50),
        "latency_p95_ms": percentile(latencies, 0.95),
        "latency_avg_ms": statistics.fmean(latencies),
    }


def benchmark_embedding_candidate(
    candidate: dict[str, Any],
    dataset: dict[str, Any],
    base_url: str,
    output_dir: Path,
) -> dict[str, Any]:
    child = None
    candidate_dir = output_dir / f"embedding-{candidate['label']}"
    actual_base_url = reserve_base_url(base_url)
    try:
        child = launch_xavier2(
            actual_base_url,
            candidate_dir,
            {
                "XAVIER2_DISABLE_HYDE": "1",
                "XAVIER2_MODEL_PROVIDER": "disabled",
                "XAVIER2_EMBEDDING_URL": candidate["url"],
                "XAVIER2_EMBEDDING_MODEL": candidate["model"],
            },
        )
        wait_for_health(actual_base_url)
        documents_by_path = reset_and_seed_memory(actual_base_url, dataset)
        records = []
        request_failures = 0
        for case in dataset.get("cases", []):
            try:
                if case["endpoint"] == "search":
                    records.append(evaluate_search_case(actual_base_url, case))
                else:
                    records.append(
                        evaluate_generation_case(
                            actual_base_url,
                            case,
                            force_system3_mode="disabled",
                        )
                    )
            except Exception as error:
                request_failures += 1
                records.append(
                    {
                        "id": case["id"],
                        "kind": "search" if case["endpoint"] == "search" else "generation",
                        "success": False,
                        "latency_ms": 0.0,
                        "error": str(error),
                    }
                )
        search_records = [record for record in records if record["kind"] == "search"]
        generation_records = [record for record in records if record["kind"] == "generation"]
        result = {
            "candidate_type": "embedding",
            "label": candidate["label"],
            "model": candidate["model"],
            "url": candidate["url"],
            "evaluated_at": utc_now(),
            "search_metrics": compute_search_metrics(search_records, documents_by_path),
            "generation_metrics": compute_generation_metrics(generation_records),
            "request_failures": request_failures,
            **summarize_latencies(records),
            "records": records,
        }
        write_json(candidate_dir / "result.json", result)
        return result
    finally:
        stop_process(child)


def configure_llm_env(candidate: dict[str, Any]) -> dict[str, str]:
    provider = candidate.get("provider", "local").strip().lower()
    if provider == "local":
        return {
            "XAVIER2_MODEL_PROVIDER": "local",
            "XAVIER2_LOCAL_LLM_URL": candidate["url"],
            "XAVIER2_LOCAL_LLM_MODEL": candidate["model"],
        }
    if provider in {"gemini", "openai", "minimax", "anthropic"}:
        return {
            "XAVIER2_MODEL_PROVIDER": provider,
            "XAVIER2_LLM_MODEL": candidate["model"],
        }
    raise ValueError(f"Unsupported LLM provider '{provider}'")


def benchmark_llm_candidate(
    candidate: dict[str, Any],
    baseline_embedding: dict[str, Any] | None,
    dataset: dict[str, Any],
    base_url: str,
    output_dir: Path,
) -> dict[str, Any]:
    child = None
    candidate_dir = output_dir / f"llm-{candidate['label']}"
    actual_base_url = reserve_base_url(base_url)
    env = {
        "XAVIER2_DISABLE_HYDE": "1",
    }
    env.update(configure_llm_env(candidate))
    if baseline_embedding:
        env["XAVIER2_EMBEDDING_URL"] = baseline_embedding["url"]
        env["XAVIER2_EMBEDDING_MODEL"] = baseline_embedding["model"]

    try:
        child = launch_xavier2(actual_base_url, candidate_dir, env)
        wait_for_health(actual_base_url)
        reset_and_seed_memory(actual_base_url, dataset)
        records = []
        request_failures = 0
        for case in dataset.get("cases", []):
            if case["endpoint"] == "search":
                continue
            try:
                records.append(
                    evaluate_generation_case(
                        actual_base_url,
                        case,
                        force_system3_mode="required",
                    )
                )
            except Exception as error:
                request_failures += 1
                records.append(
                    {
                        "id": case["id"],
                        "kind": "generation",
                        "success": False,
                        "latency_ms": 0.0,
                        "citation_like": False,
                        "error": str(error),
                        "endpoint": case["endpoint"],
                    }
                )
        result = {
            "candidate_type": "llm",
            "label": candidate["label"],
            "provider": candidate.get("provider", "local"),
            "model": candidate["model"],
            "url": candidate.get("url"),
            "evaluated_at": utc_now(),
            "generation_metrics": compute_generation_metrics(records),
            "request_failures": request_failures,
            **summarize_latencies(records),
            "records": records,
        }
        write_json(candidate_dir / "result.json", result)
        return result
    finally:
        stop_process(child)


def choose_baseline_embedding(config: dict[str, Any]) -> dict[str, Any] | None:
    candidates = config.get("embedding_candidates", [])
    if not candidates:
        return None
    baseline_label = config.get("baseline_embedding_label")
    if baseline_label:
        for candidate in candidates:
            if candidate["label"] == baseline_label:
                return candidate
    return candidates[0]


def command_benchmark(args: argparse.Namespace) -> None:
    config = load_json(args.config)
    dataset = load_json(config.get("dataset", DEFAULT_DATASET))
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    base_url = config.get("base_url", DEFAULT_BASE_URL)
    results = {
        "generated_at": utc_now(),
        "config": config,
        "embedding_candidates": [],
        "llm_candidates": [],
    }

    for candidate in config.get("embedding_candidates", []):
        results["embedding_candidates"].append(
            benchmark_embedding_candidate(candidate, dataset, base_url, output_dir)
        )

    baseline_embedding = choose_baseline_embedding(config)
    for candidate in config.get("llm_candidates", []):
        results["llm_candidates"].append(
            benchmark_llm_candidate(candidate, baseline_embedding, dataset, base_url, output_dir)
        )

    write_json(output_dir / "model_eval.json", results)
    print(json.dumps(results, indent=2))


def normalize_priority(value: str | None) -> str:
    normalized = (value or "medium").strip().lower()
    if normalized in {"critical", "high", "medium", "low", "ephemeral"}:
        return normalized
    return "medium"


def priority_score(value: str) -> int:
    return {
        "critical": 4,
        "high": 3,
        "medium": 2,
        "low": 1,
        "ephemeral": 0,
    }.get(normalize_priority(value), 2)


def low_signal(content: str) -> bool:
    text = content.strip().lower()
    if len(text) < 24:
        return True
    return text in {"ok", "okay", "thanks", "gracias", "hello", "hola"}


def fetch_all_memories(base_url: str) -> list[dict[str, Any]]:
    offset = 0
    limit = 100
    memories = []
    while True:
        payload = http_json(f"{base_url}/v1/memories?limit={limit}&offset={offset}")
        batch = payload.get("memories", [])
        memories.extend(batch)
        if len(batch) < limit:
            break
        offset += limit
    return memories


def load_tasks(path: str | None) -> list[dict[str, Any]]:
    if not path:
        return []
    payload = load_json(path)
    if isinstance(payload, list):
        return payload
    return payload.get("tasks", [])


def boost_from_tasks(memory: dict[str, Any], tasks: list[dict[str, Any]]) -> int:
    metadata = memory.get("metadata", {})
    namespace = metadata.get("namespace", {}) if isinstance(metadata, dict) else {}
    project = namespace.get("project") or metadata.get("project")
    text = f"{memory.get('memory', '')} {memory.get('id', '')}".lower()
    boost = 0
    for task in tasks:
        priority = str(task.get("priority", "medium")).lower()
        if priority not in {"high", "urgent"}:
            continue
        if project and project == task.get("project"):
            boost = max(boost, 2)
        title = str(task.get("title", "")).lower()
        if title and any(token in text for token in title.split()[:3]):
            boost = max(boost, 1)
    return boost


def deterministic_split(identifier: str) -> str:
    digest = hashlib.sha1(identifier.encode("utf-8")).hexdigest()
    bucket = int(digest[:2], 16) % 10
    if bucket == 0:
        return "eval"
    if bucket == 1:
        return "validation"
    return "train"


def generate_query(memory: dict[str, Any]) -> str:
    metadata = memory.get("metadata", {})
    title = metadata.get("title") if isinstance(metadata, dict) else None
    namespace = metadata.get("namespace", {}) if isinstance(metadata, dict) else {}
    project = namespace.get("project") or metadata.get("project")
    content = str(memory.get("memory", "")).strip()
    if title:
        return f"What should Xavier2 retrieve about {title}?"
    sentence = content.split(".")[0].strip()
    if project:
        return f"What does Xavier2 know about {project}: {sentence[:96]}?"
    return f"What memory matches this fact: {sentence[:96]}?"


def sample_negatives(
    current: dict[str, Any],
    candidates: list[dict[str, Any]],
    limit: int = 3,
) -> list[str]:
    current_meta = current.get("metadata", {})
    current_kind = current_meta.get("kind")
    negatives = []
    for candidate in candidates:
        if candidate["id"] == current["id"]:
            continue
        candidate_meta = candidate.get("metadata", {})
        same_kind = candidate_meta.get("kind") == current_kind
        if same_kind:
            continue
        negatives.append(candidate["id"])
        if len(negatives) >= limit:
            break
    return negatives


def build_training_bundle(memories: list[dict[str, Any]], tasks: list[dict[str, Any]]) -> dict[str, Any]:
    selected = []
    coverage = Counter()
    for index, memory in enumerate(memories):
        memory = dict(memory)
        if not memory.get("id"):
            memory["id"] = memory.get("user_id") or f"memory-{index}"
        content = str(memory.get("memory", "")).strip()
        metadata = memory.get("metadata", {})
        priority = normalize_priority(metadata.get("memory_priority") if isinstance(metadata, dict) else None)
        if priority == "ephemeral" or low_signal(content):
            continue
        score = priority_score(priority) + boost_from_tasks(memory, tasks)
        if score < 2:
            continue
        memory["effective_priority"] = priority
        memory["selection_score"] = score
        selected.append(memory)
        coverage[priority] += 1

    selected.sort(
        key=lambda item: (
            -item["selection_score"],
            -priority_score(item["effective_priority"]),
            item["id"],
        )
    )

    examples = []
    for memory in selected:
        split = deterministic_split(memory["id"])
        examples.append(
            {
                "id": memory["id"],
                "split": split,
                "query": generate_query(memory),
                "positive_id": memory["id"],
                "positive_text": memory["memory"],
                "negative_ids": sample_negatives(memory, selected),
                "priority": memory["effective_priority"],
                "metadata": memory.get("metadata", {}),
            }
        )

    report = {
        "generated_at": utc_now(),
        "source_memories": len(memories),
        "selected_memories": len(selected),
        "coverage_by_priority": dict(coverage),
        "split_counts": dict(Counter(example["split"] for example in examples)),
        "tasks_considered": len(tasks),
        "target": "embeddings",
    }
    return {
        "documents": selected,
        "examples": examples,
        "report": report,
    }


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record) + "\n")


def command_export_training(args: argparse.Namespace) -> None:
    base_url = args.base_url or DEFAULT_BASE_URL
    memories = fetch_all_memories(base_url)
    tasks = load_tasks(args.tasks_file)
    bundle = build_training_bundle(memories, tasks)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    examples = bundle["examples"]
    write_jsonl(output_dir / "train.jsonl", [item for item in examples if item["split"] == "train"])
    write_jsonl(
        output_dir / "validation.jsonl",
        [item for item in examples if item["split"] == "validation"],
    )
    write_jsonl(output_dir / "eval.jsonl", [item for item in examples if item["split"] == "eval"])
    write_jsonl(output_dir / "gold_queries.jsonl", examples)
    write_jsonl(output_dir / "documents.jsonl", bundle["documents"])
    write_json(output_dir / "bundle_manifest.json", bundle["report"])
    print(json.dumps(bundle["report"], indent=2))


def rank_embedding_candidates(
    candidates: list[dict[str, Any]],
    baseline_label: str | None,
) -> tuple[dict[str, Any] | None, list[dict[str, Any]]]:
    baseline = None
    for candidate in candidates:
        if candidate["label"] == baseline_label:
            baseline = candidate
            break
    if baseline is None and candidates:
        baseline = candidates[0]
    if baseline is None:
        return None, []

    baseline_search = baseline.get("search_metrics", {})
    selected = baseline
    ranking = []
    for candidate in candidates:
        metrics = candidate.get("search_metrics", {})
        high_priority = metrics.get("high_priority_recall_at_10")
        baseline_high = baseline_search.get("high_priority_recall_at_10")
        if candidate.get("request_failures", 0) > 0:
            candidate["composite_score"] = -1.0
            ranking.append(candidate)
            continue
        improved = (
            high_priority is not None
            and baseline_high is not None
            and high_priority >= baseline_high
            and metrics.get("ndcg_at_10", 0.0) >= baseline_search.get("ndcg_at_10", 0.0)
        )
        composite = (
            metrics.get("recall_at_10", 0.0) * 0.45
            + metrics.get("ndcg_at_10", 0.0) * 0.35
            + (high_priority or 0.0) * 0.20
        )
        candidate["composite_score"] = composite
        ranking.append(candidate)
        if improved and composite > selected.get("composite_score", -1.0):
            selected = candidate
    ranking.sort(key=lambda item: item.get("composite_score", 0.0), reverse=True)
    return selected, ranking


def rank_llm_candidates(
    candidates: list[dict[str, Any]],
    fast_latency_budget_ms: float,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None, list[dict[str, Any]]]:
    if not candidates:
        return None, None, []

    ranking = []
    for candidate in candidates:
        metrics = candidate.get("generation_metrics", {})
        if candidate.get("request_failures", 0) > 0:
            candidate["composite_score"] = -1.0
            ranking.append(candidate)
            continue
        composite = (
            metrics.get("faithfulness_score", 0.0) * 0.70
            + metrics.get("citation_score", 0.0) * 0.20
            + (1.0 / max(candidate.get("latency_p95_ms", 1.0), 1.0)) * 0.10
        )
        candidate["composite_score"] = composite
        ranking.append(candidate)

    ranking.sort(
        key=lambda item: (
            item.get("request_failures", 0) == 0,
            item.get("generation_metrics", {}).get("faithfulness_score", 0.0),
            -item.get("latency_p95_ms", 10_000.0),
        ),
        reverse=True,
    )
    quality = ranking[0]

    fast = None
    quality_score = quality.get("generation_metrics", {}).get("faithfulness_score", 0.0)
    for candidate in sorted(ranking, key=lambda item: item.get("latency_p95_ms", 10_000.0)):
        if candidate.get("request_failures", 0) > 0:
            continue
        faithfulness = candidate.get("generation_metrics", {}).get("faithfulness_score", 0.0)
        if faithfulness + 0.05 < quality_score:
            continue
        if candidate.get("latency_p95_ms", 10_000.0) <= fast_latency_budget_ms:
            fast = candidate
            break
    if fast is None:
        fast = quality
    return fast, quality, ranking


def command_publish_policy(args: argparse.Namespace) -> None:
    evaluation = load_json(args.evaluation)
    output_path = Path(args.output)
    baseline_embedding_label = args.baseline_embedding_label
    selected_embedding, ranked_embeddings = rank_embedding_candidates(
        evaluation.get("embedding_candidates", []),
        baseline_embedding_label,
    )
    fast_model, quality_model, ranked_llms = rank_llm_candidates(
        evaluation.get("llm_candidates", []),
        args.fast_latency_budget_ms,
    )

    single_model = (
        not fast_model
        or not quality_model
        or fast_model["label"] == quality_model["label"]
        or quality_model.get("generation_metrics", {}).get("faithfulness_score", 0.0) <= 0.0
    )

    policy = {
        "version": 1,
        "generated_at": utc_now(),
        "selection": {
            "single_model_mode": single_model,
            "fast_label": fast_model["label"] if fast_model else None,
            "quality_label": None if single_model or not quality_model else quality_model["label"],
            "selected_embedding": selected_embedding["label"] if selected_embedding else None,
        },
        "models": {
            "fast": None
            if not fast_model
            else {
                "name": fast_model["model"],
                "enabled": True,
                "benchmark_latency_ms": round(fast_model.get("latency_p95_ms", 0.0)),
                "quality_score": fast_model.get("generation_metrics", {}).get(
                    "faithfulness_score", 0.0
                ),
            },
            "quality": None
            if single_model or not quality_model
            else {
                "name": quality_model["model"],
                "enabled": True,
                "benchmark_latency_ms": round(quality_model.get("latency_p95_ms", 0.0)),
                "quality_score": quality_model.get("generation_metrics", {}).get(
                    "faithfulness_score", 0.0
                ),
            },
        },
        "thresholds": {
            "strong_retrieval_confidence": 0.72,
            "weak_reasoning_confidence": 0.68,
        },
        "embeddings": [
            {
                "label": candidate["label"],
                "model": candidate["model"],
                "url": candidate.get("url"),
                "selected": selected_embedding is not None
                and candidate["label"] == selected_embedding["label"],
                "recall_at_5": candidate.get("search_metrics", {}).get("recall_at_5"),
                "recall_at_10": candidate.get("search_metrics", {}).get("recall_at_10"),
                "ndcg_at_10": candidate.get("search_metrics", {}).get("ndcg_at_10"),
            }
            for candidate in ranked_embeddings
        ],
        "ranking": {
            "llm": [
                {
                    "label": candidate["label"],
                    "model": candidate["model"],
                    "faithfulness_score": candidate.get("generation_metrics", {}).get(
                        "faithfulness_score", 0.0
                    ),
                    "latency_p95_ms": candidate.get("latency_p95_ms", 0.0),
                }
                for candidate in ranked_llms
            ],
            "embedding": [
                {
                    "label": candidate["label"],
                    "model": candidate["model"],
                    "recall_at_10": candidate.get("search_metrics", {}).get("recall_at_10", 0.0),
                    "ndcg_at_10": candidate.get("search_metrics", {}).get("ndcg_at_10", 0.0),
                }
                for candidate in ranked_embeddings
            ],
        },
    }
    write_json(output_path, policy)
    print(json.dumps(policy, indent=2))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Unified benchmark, training export, and router policy publisher for Xavier2."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    benchmark = subparsers.add_parser("benchmark")
    benchmark.add_argument("--config", required=True, help="Path to benchmark config JSON.")
    benchmark.add_argument("--output-dir", required=True, help="Directory for benchmark outputs.")
    benchmark.set_defaults(func=command_benchmark)

    export_training = subparsers.add_parser("export-training")
    export_training.add_argument(
        "--base-url",
        default=DEFAULT_BASE_URL,
        help="Xavier2 base URL used to fetch primary memories.",
    )
    export_training.add_argument(
        "--tasks-file",
        default=None,
        help="Optional JSON file with tasks to boost bundle selection.",
    )
    export_training.add_argument("--output-dir", required=True, help="Training bundle directory.")
    export_training.set_defaults(func=command_export_training)

    publish = subparsers.add_parser("publish-policy")
    publish.add_argument("--evaluation", required=True, help="Path to model_eval.json.")
    publish.add_argument("--output", required=True, help="Output path for model_policy.json.")
    publish.add_argument(
        "--baseline-embedding-label",
        default=None,
        help="Optional label that identifies the embedding baseline.",
    )
    publish.add_argument(
        "--fast-latency-budget-ms",
        type=float,
        default=1500.0,
        help="Maximum p95 latency for the fast model candidate.",
    )
    publish.set_defaults(func=command_publish_policy)

    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
