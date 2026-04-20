#!/usr/bin/env python3
"""Smoke test for Termux Docker runtime."""

from __future__ import annotations

import json
import os
import platform
import shutil
from pathlib import Path


def main() -> None:
    workspace = Path("/workspace")
    src = workspace / "src"
    rust_files = sorted(src.rglob("*.rs")) if src.exists() else []
    sample = []
    for path in rust_files[:5]:
        sample.append(str(path.relative_to(workspace)))

    payload = {
        "status": "ok",
        "runtime": "termux-docker",
        "python": platform.python_version(),
        "platform": platform.platform(),
        "cwd": os.getcwd(),
        "workspace_exists": workspace.exists(),
        "rust_file_count": len(rust_files),
        "sample_files": sample,
        "tools": {
            "bash": shutil.which("bash"),
            "git": shutil.which("git"),
            "python": shutil.which("python"),
            "rg": shutil.which("rg"),
        },
    }
    print(json.dumps(payload, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
