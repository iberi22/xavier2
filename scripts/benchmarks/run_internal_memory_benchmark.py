import argparse
import json
import os
import socket
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DATASET = ROOT / "scripts" / "benchmarks" / "datasets" / "internal_swal_openclaw_memory.json"
HTTP_TIMEOUT_SECONDS = 60


def http_json(url: str, payload: dict, method: str = "POST") -> dict:
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={
            "Content-Type": "application/json",
            "X-Xavier2-Token": "dev-token",
        },
    )
    with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(base_url: str) -> None:
    for _ in range(60):
        try:
            with urllib.request.urlopen(f"{base_url}/health", timeout=5) as response:
                if response.status == 200:
                    return
        except Exception:
            time.sleep(1)
    raise RuntimeError("Xavier2 did not become healthy in time")


def add_documents(base_url: str, dataset: dict) -> None:
    http_json(f"{base_url}/memory/reset", {})
    for document in dataset["documents"]:
        payload = {
            "path": document["path"],
            "content": document["content"],
            "metadata": document.get("metadata", {}),
            "kind": document.get("kind"),
            "evidence_kind": document.get("evidence_kind"),
            "namespace": document.get("namespace"),
            "provenance": document.get("provenance"),
        }
        http_json(f"{base_url}/memory/add", payload)


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


def evaluate_case(base_url: str, case: dict) -> dict:
    endpoint = case["endpoint"]
    payload = {
        "query": case["query"],
        "limit": 5,
        "filters": case.get("filters"),
    }
    if "system3_mode" in case:
        payload["system3_mode"] = case["system3_mode"]
    if endpoint == "search":
        response = http_json(f"{base_url}/memory/search", payload)
        top_path = None
        if response.get("results"):
            top_path = response["results"][0].get("path")
        return {
            "id": case["id"],
            "endpoint": endpoint,
            "success": top_path == case["expected_path"],
            "expected_path": case["expected_path"],
            "actual_path": top_path,
        }

    route = "/memory/query" if endpoint == "query" else "/agents/run"
    response = http_json(f"{base_url}{route}", payload)
    answer = response.get("response", "")
    expected = case["expected_substring"]
    return {
        "id": case["id"],
        "endpoint": endpoint,
        "success": expected.lower() in answer.lower(),
        "expected_substring": expected,
        "actual_response": answer,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="http://127.0.0.1:8003")
    parser.add_argument("--dataset", default=str(DEFAULT_DATASET))
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--use-existing-server", action="store_true")
    args = parser.parse_args()

    base_url = args.base_url
    if not args.use_existing_server:
        base_url = reserve_base_url(base_url)

    dataset = json.loads(Path(args.dataset).read_text(encoding="utf-8"))
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    child = None
    if not args.use_existing_server:
        parsed = urllib.parse.urlparse(base_url)
        env = os.environ.copy()
        if parsed.hostname:
            env["XAVIER2_HOST"] = parsed.hostname
        if parsed.port:
            env["XAVIER2_PORT"] = str(parsed.port)
        # Keep the internal suite deterministic and evidence-first.
        env["XAVIER2_DISABLE_HYDE"] = "1"
        env["XAVIER2_MODEL_PROVIDER"] = "disabled"
        child = subprocess.Popen(
            ["cargo", "run", "--bin", "xavier2"],
            cwd=ROOT,
            env=env,
            stdout=(output_dir / "xavier2.stdout.log").open("wb"),
            stderr=(output_dir / "xavier2.stderr.log").open("wb"),
        )

    try:
        wait_for_health(base_url)
        add_documents(base_url, dataset)
        records = [evaluate_case(base_url, case) for case in dataset["cases"]]
        summary = {
            "benchmark": "internal_swal_openclaw_memory",
            "dataset": str(Path(args.dataset)),
            "base_url": base_url,
            "cases": len(records),
            "passed": sum(1 for record in records if record["success"]),
            "accuracy": sum(1 for record in records if record["success"]) / len(records),
        }
        (output_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        (output_dir / "records.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
        print(json.dumps({"summary": summary, "records": records}, indent=2))
    finally:
        if child is not None:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()


if __name__ == "__main__":
    main()
