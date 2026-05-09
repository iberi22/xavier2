#!/usr/bin/env python3
import argparse
import difflib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from collections import defaultdict
from pathlib import Path


LOCOMO_REPO = "https://github.com/snap-research/LoCoMo.git"
REPO_ROOT = Path(__file__).resolve().parents[2]


def run(cmd, cwd=None, env=None):
    subprocess.run(cmd, cwd=cwd, env=env, check=True)


def clone_or_update(repo_url: str, name: str) -> Path:
    override_dir = os.environ.get("XAVIER_BENCHMARK_CACHE_DIR", "").strip()
    base = (
        Path(override_dir).expanduser()
        if override_dir
        else Path(tempfile.gettempdir()) / "xavier-benchmark-sources"
    )
    base.mkdir(parents=True, exist_ok=True)
    target = base / name
    if target.exists():
        run(["git", "-C", str(target), "fetch", "--all", "--prune"])
    else:
        run(["git", "clone", repo_url, str(target)])
    return target


def resolve_xavier_binary(raw_path: str) -> Path:
    if raw_path:
        candidate = Path(raw_path).expanduser()
        if not candidate.is_absolute():
            candidate = (REPO_ROOT / candidate).resolve()
        if candidate.is_file():
            return candidate
        raise FileNotFoundError(
            f"Xavier binary not found at '{candidate}'. "
            f"Build it first with `cargo build --release --bin xavier` "
            f"or pass --xavier-binary/ XAVIER_BINARY with the absolute path."
        )

    env_path = os.environ.get("XAVIER_BINARY", "").strip()
    if env_path:
        return resolve_xavier_binary(env_path)

    names = ["xavier.exe", "xavier"] if os.name == "nt" else ["xavier", "xavier.exe"]
    search_roots = [
        REPO_ROOT / "target" / "release",
        Path.home() / ".cargo" / "target_global" / "release",
    ]

    for root in search_roots:
        for name in names:
            candidate = root / name
            if candidate.is_file():
                return candidate.resolve()

    searched = ", ".join(str(root) for root in search_roots)
    raise FileNotFoundError(
        "Unable to locate Xavier binary automatically. "
        f"Searched: {searched}. "
        "Build it first with `cargo build --release --bin xavier` or set XAVIER_BINARY."
    )


def http_json(url: str, payload=None, method="GET", timeout=30):
    headers = {"Content-Type": "application/json"}
    token = os.environ.get("XAVIER_TOKEN", "").strip()
    if token:
        headers["X-Xavier-Token"] = token
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(base_url: str, timeout_seconds: int = 60) -> None:
    deadline = time.time() + timeout_seconds
    last_error = None
    while time.time() < deadline:
        try:
            payload = http_json(f"{base_url}/health")
            if payload.get("status") == "ok":
                return
        except Exception as error:  # pragma: no cover - integration only
            last_error = error
        time.sleep(1)
    raise RuntimeError(f"Xavier health check failed: {last_error}")


def find_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        sock.listen(1)
        return sock.getsockname()[1]


def normalize_text(text) -> str:
    text = str(text)
    text = text.lower()
    text = re.sub(r"\b(a|an|the)\b", " ", text)
    text = re.sub(r"[^a-z0-9\s]", " ", text)
    return " ".join(text.split())


def token_f1(prediction: str, answer: str) -> float:
    pred_tokens = normalize_text(prediction).split()
    answer_tokens = normalize_text(answer).split()
    if not pred_tokens and not answer_tokens:
        return 1.0
    if not pred_tokens or not answer_tokens:
        return 0.0

    pred_counts = defaultdict(int)
    answer_counts = defaultdict(int)
    for token in pred_tokens:
        pred_counts[token] += 1
    for token in answer_tokens:
        answer_counts[token] += 1

    overlap = sum(min(pred_counts[token], answer_counts[token]) for token in pred_counts)
    if overlap == 0:
        return 0.0
    precision = overlap / len(pred_tokens)
    recall = overlap / len(answer_tokens)
    return 2 * precision * recall / (precision + recall)


def exact_match(prediction: str, answer: str) -> float:
    return float(normalize_text(prediction) == normalize_text(answer))


def normalize_dia_id(value) -> str:
    text = str(value or "").strip()
    if not text:
        return ""
    match = re.match(r"(?i)^([a-z]+\d+):0*([0-9]+)$", text)
    if match:
        return f"{match.group(1).upper()}:{int(match.group(2))}"
    return text.upper()


def normalize_evidence(evidence) -> list[str]:
    normalized = []
    for item in evidence or []:
        if isinstance(item, dict):
            dia_id = item.get("dia_id") or item.get("id")
            if dia_id:
                normalized.append(normalize_dia_id(dia_id))
        else:
            normalized.append(normalize_dia_id(item))
    return [item for item in normalized if item]


def check_semantic_equivalence(base_url: str, prediction: str, answer: str) -> bool:
    prompt = (
        f"Are these two answers semantically equivalent in the context of a conversation? "
        f"Answer only 'yes' or 'no'.\n\n"
        f"Answer 1: {prediction}\n"
        f"Answer 2: {answer}"
    )
    try:
        response = http_json(
            f"{base_url}/agents/run",
            {"query": prompt},
            method="POST",
        )
        result = response.get("response", "").strip().lower()
        return "yes" in result
    except Exception:
        return False


def evaluate_date(prediction: str, answer: str) -> float:
    """Category 2: Dates (Exact match only)"""
    return exact_match(prediction, answer)


def evaluate_opinion(base_url: str, prediction: str, answer: str) -> float:
    """Category 3: Opinions (Semantic equivalence)"""
    if exact_match(prediction, answer) > 0.9:
        return 1.0
    return float(check_semantic_equivalence(base_url, prediction, answer))


def evaluate_action(prediction: str, answer: str) -> float:
    """Category 4: Actions (Partial match OK with 0.7 threshold)"""
    ratio = difflib.SequenceMatcher(None, prediction.lower(), answer.lower()).ratio()
    return float(ratio >= 0.7)


def evaluate(base_url: str, prediction: str, answer: str, category: any) -> float:
    # Convert to string for safety - prediction/answer might be int from API
    prediction = str(prediction) if prediction is not None else ""
    answer = str(answer) if answer is not None else ""

    cat_str = str(category)
    if cat_str == "2":
        return evaluate_date(prediction, answer)
    if cat_str == "3":
        return evaluate_opinion(base_url, prediction, answer)
    if cat_str == "4":
        return evaluate_action(prediction, answer)

    # Others: Default 0.85 threshold
    ratio = difflib.SequenceMatcher(None, prediction.lower(), answer.lower()).ratio()
    return float(ratio >= 0.85)


def session_keys(conversation: dict) -> list[str]:
    return sorted(
        [
            key
            for key in conversation
            if key.startswith("session_") and not key.endswith("_date_time")
        ],
        key=lambda key: int(key.split("_")[1]),
    )


def add_conversation(base_url: str, sample: dict) -> int:
    added = 0
    conversation = sample["conversation"]
    observations = sample.get("observation", {})
    session_summaries = sample.get("session_summary", {})

    for session_key in session_keys(conversation):
        session = conversation.get(session_key, [])
        session_time = conversation.get(f"{session_key}_date_time")
        for turn in session:
            dia_id = normalize_dia_id(turn.get("dia_id", f"{session_key}-{added}"))
            speaker = turn.get("speaker", "unknown")
            content = turn.get("text", "")
            path = f"locomo/{sample['sample_id']}/{session_key}/{dia_id}"
            metadata = {
                "benchmark": "locomo",
                "sample_id": sample["sample_id"],
                "session": session_key,
                "session_time": session_time,
                "speaker": speaker,
                "dia_id": dia_id,
                "category": "conversation",
            }
            if turn.get("img_url"):
                metadata["img_url"] = turn["img_url"]
            if turn.get("blip_caption"):
                metadata["blip_caption"] = turn["blip_caption"]
            http_json(
                f"{base_url}/memory/add",
                {
                    "path": path,
                    "content": f"{speaker}: {content}",
                    "metadata": metadata,
                },
                method="POST",
            )
            added += 1

        observation_key = f"{session_key}_observation"
        for index, observation in enumerate(observations.get(observation_key, [])):
            http_json(
                f"{base_url}/memory/add",
                {
                    "path": f"locomo/{sample['sample_id']}/{session_key}/observation/{index}",
                    "content": str(observation),
                    "metadata": {
                        "benchmark": "locomo",
                        "sample_id": sample["sample_id"],
                        "session": session_key,
                        "session_time": session_time,
                        "category": "observation",
                    },
                },
                method="POST",
            )
            added += 1

        summary_key = f"{session_key}_summary"
        summary = session_summaries.get(summary_key)
        if summary:
            http_json(
                f"{base_url}/memory/add",
                {
                    "path": f"locomo/{sample['sample_id']}/{session_key}/summary",
                    "content": str(summary),
                    "metadata": {
                        "benchmark": "locomo",
                        "sample_id": sample["sample_id"],
                        "session": session_key,
                        "session_time": session_time,
                        "category": "session_summary",
                    },
                },
                method="POST",
            )
            added += 1
    return added


def score_predictions(records: list[dict]) -> dict:
    categories = defaultdict(
        lambda: {"count": 0, "exact_match": 0.0, "token_f1": 0.0, "accuracy": 0.0}
    )
    summary = {"count": 0, "exact_match": 0.0, "token_f1": 0.0, "accuracy": 0.0}

    for record in records:
        category = record["category"]

        # Category 5: Adversarial (Impossible questions) - exclude from overall accuracy
        if str(category) != "5":
            summary["count"] += 1
            summary["exact_match"] += record["exact_match"]
            summary["token_f1"] += record["token_f1"]
            summary["accuracy"] += record.get("is_correct", 0.0)

        categories[category]["count"] += 1
        categories[category]["exact_match"] += record["exact_match"]
        categories[category]["token_f1"] += record["token_f1"]
        categories[category]["accuracy"] += record.get("is_correct", 0.0)

    if summary["count"]:
        summary["exact_match"] /= summary["count"]
        summary["token_f1"] /= summary["count"]
        summary["accuracy"] /= summary["count"]

    category_metrics = {}
    for category, metrics in categories.items():
        if metrics["count"]:
            category_metrics[category] = {
                "count": metrics["count"],
                "exact_match": metrics["exact_match"] / metrics["count"],
                "token_f1": metrics["token_f1"] / metrics["count"],
                "accuracy": metrics["accuracy"] / metrics["count"],
            }

    return {"overall": summary, "by_category": category_metrics}


def resolve_modes(args) -> list[str]:
    if args.strict_no_hints:
        return ["strict"]
    if args.mode == "both":
        return ["assisted", "strict"]
    return [args.mode]


def build_query_payload(question: str, category, mode: str) -> dict:
    payload = {"query": question, "limit": 10}
    if mode == "assisted":
        payload["category"] = str(category)
    return payload


def evaluate_mode(base_url: str, samples: list[dict], args, mode: str) -> tuple[list[dict], dict]:
    records = []
    for sample in samples:
        http_json(f"{base_url}/memory/reset", {}, method="POST")
        added = add_conversation(base_url, sample)

        questions = sample.get("qa", [])
        if args.question_limit > 0:
            questions = questions[: args.question_limit]

        for index, qa in enumerate(questions):
            question = qa.get("question")
            answer = qa.get("answer")
            if not question or answer is None:
                continue
            category = qa.get("category", "unknown")
            payload = http_json(
                f"{base_url}/memory/query",
                build_query_payload(question, category, mode),
                method="POST",
            )
            prediction = payload.get("response", "").strip()
            evidence = qa.get("evidence", [])
            record = {
                "sample_id": sample["sample_id"],
                "question_index": index,
                "question": question,
                "answer": answer,
                "prediction": prediction,
                "category": category,
                "mode": mode,
                "evidence": evidence,
                "normalized_evidence": normalize_evidence(evidence),
                "documents_ingested": added,
                "exact_match": exact_match(prediction, answer),
                "token_f1": token_f1(prediction, answer),
                "is_correct": evaluate(base_url, prediction, answer, category),
            }
            records.append(record)

    return records, score_predictions(records)


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--xavier-binary", default="")
    parser.add_argument("--base-url", default="http://127.0.0.1:8003")
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--sample-limit", type=int, default=10)
    parser.add_argument("--question-limit", type=int, default=0)
    parser.add_argument("--use-existing-server", action="store_true")
    parser.add_argument(
        "--mode",
        choices=["assisted", "strict", "both"],
        default="both",
    )
    parser.add_argument("--strict-no-hints", action="store_true")
    return parser.parse_args()


def main():
    args = parse_args()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    base_url = args.base_url
    modes = resolve_modes(args)

    locomo_repo = clone_or_update(LOCOMO_REPO, "LoCoMo")
    dataset_path = locomo_repo / "data" / "locomo10.json"
    samples = json.loads(dataset_path.read_text(encoding="utf-8"))[: args.sample_limit]

    child = None
    child_stdout = output_dir / "xavier.stdout.log"
    child_stderr = output_dir / "xavier.stderr.log"
    if not args.use_existing_server:
        xavier_binary = resolve_xavier_binary(args.xavier_binary)
        port = find_free_port()
        env = os.environ.copy()
        env["XAVIER_DEV_MODE"] = "1"
        env["XAVIER_HOST"] = "127.0.0.1"
        env["XAVIER_PORT"] = str(port)
        env["XAVIER_CODE_GRAPH_DB_PATH"] = str(output_dir / "code_graph.db")
        base_url = f"http://127.0.0.1:{port}"
        child = subprocess.Popen(
            [str(xavier_binary)],
            cwd=REPO_ROOT,
            env=env,
            stdout=child_stdout.open("wb"),
            stderr=child_stderr.open("wb"),
        )

    try:
        wait_for_health(base_url)

        mode_results = {}
        for mode in modes:
            records, metrics = evaluate_mode(base_url, samples, args, mode)
            mode_results[mode] = {
                "mode": mode,
                "strict_no_hints": mode == "strict",
                "questions_evaluated": len(records),
                "metrics": metrics,
            }
            (output_dir / f"predictions-{mode}.json").write_text(
                json.dumps(records, indent=2), encoding="utf-8"
            )
            if len(modes) == 1:
                (output_dir / "predictions.json").write_text(
                    json.dumps(records, indent=2), encoding="utf-8"
                )

        summary = {
            "benchmark": "locomo",
            "dataset": str(dataset_path),
            "samples_evaluated": len(samples),
            "modes": mode_results,
        }
        if len(modes) == 1:
            only_mode = modes[0]
            summary.update(mode_results[only_mode])

        (output_dir / "summary.json").write_text(
            json.dumps(summary, indent=2), encoding="utf-8"
        )
        print(json.dumps(summary, indent=2))
    finally:
        if child is not None:
            child.terminate()
            try:
                child.wait(timeout=15)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=15)


if __name__ == "__main__":
    sys.exit(main())
