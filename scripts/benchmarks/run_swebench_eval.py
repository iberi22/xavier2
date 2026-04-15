#!/usr/bin/env python3
import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


SWE_BENCH_REPO = "https://github.com/swe-bench/SWE-bench.git"
MIN_FREE_GB = 120


def run(cmd, cwd=None, env=None):
    subprocess.run(cmd, cwd=cwd, env=env, check=True)


def clone_or_update(repo_url: str, name: str) -> Path:
    base = Path(tempfile.gettempdir()) / "xavier2-benchmark-sources"
    base.mkdir(parents=True, exist_ok=True)
    target = base / name
    if target.exists():
        run(["git", "-C", str(target), "fetch", "--all", "--prune"])
    else:
        run(["git", "clone", repo_url, str(target)])
    return target


def ensure_docker():
    run(["docker", "version"])


def ensure_disk_space(path: Path):
    usage = shutil.disk_usage(path)
    free_gb = usage.free / (1024 ** 3)
    if free_gb < MIN_FREE_GB:
        raise RuntimeError(
            f"SWE-bench requires about {MIN_FREE_GB} GB free; runner only has {free_gb:.2f} GB."
        )


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset-name", default="princeton-nlp/SWE-bench_Lite")
    parser.add_argument("--predictions-path", default="gold")
    parser.add_argument("--max-workers", type=int, default=1)
    parser.add_argument("--run-id", default=f"xavier2-{int(time.time())}")
    parser.add_argument("--instance-ids", default="")
    parser.add_argument("--output-dir", required=True)
    return parser.parse_args()


def main():
    args = parse_args()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    ensure_docker()
    ensure_disk_space(Path(tempfile.gettempdir()))

    swebench_repo = clone_or_update(SWE_BENCH_REPO, "SWE-bench")
    run([sys.executable, "-m", "pip", "install", "-e", "."], cwd=swebench_repo)

    cmd = [
        sys.executable,
        "-m",
        "swebench.harness.run_evaluation",
        "--dataset_name",
        args.dataset_name,
        "--predictions_path",
        args.predictions_path,
        "--max_workers",
        str(args.max_workers),
        "--run_id",
        args.run_id,
    ]
    if args.instance_ids.strip():
        cmd.extend(["--instance_ids", *[part.strip() for part in args.instance_ids.split(",") if part.strip()]])

    run(cmd, cwd=swebench_repo)

    for dirname in ["evaluation_results", "logs"]:
        source = swebench_repo / dirname
        if source.exists():
            destination = output_dir / dirname
            if destination.exists():
                shutil.rmtree(destination)
            shutil.copytree(source, destination)


if __name__ == "__main__":
    sys.exit(main())
