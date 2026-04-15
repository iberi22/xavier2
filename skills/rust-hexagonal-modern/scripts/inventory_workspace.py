#!/usr/bin/env python
from __future__ import annotations

import argparse
import tomllib
from pathlib import Path


TRACKED_CRATES = [
    "axum",
    "tokio",
    "tower-http",
    "reqwest",
    "serde",
    "serde_json",
    "thiserror",
    "tracing",
    "tracing-subscriber",
    "clap",
    "ratatui",
    "rusqlite",
    "parking_lot",
    "uuid",
    "chrono",
]

SKIP_PARTS = {
    ".git",
    "node_modules",
    "target",
    "target_bench",
    "target_bench_run1",
    "target_verify",
    ".codex-tmp",
    ".tmp-internal-8138",
}


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def normalize_dep(spec: object) -> str:
    if isinstance(spec, str):
        return spec
    if isinstance(spec, dict):
        parts: list[str] = []
        version = spec.get("version")
        if version:
            parts.append(f"version={version}")
        path = spec.get("path")
        if path:
            parts.append(f"path={path}")
        features = spec.get("features")
        if features:
            parts.append("features=" + ",".join(str(item) for item in features))
        default_features = spec.get("default-features")
        if default_features is False:
            parts.append("default-features=false")
        optional = spec.get("optional")
        if optional:
            parts.append("optional=true")
        return "; ".join(parts) if parts else "<table-spec>"
    return "<unknown>"


def collect_dependencies(data: dict) -> dict[str, str]:
    sections = [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.dependencies",
    ]
    collected: dict[str, str] = {}
    for section in sections:
        cursor = data
        for key in section.split("."):
            if not isinstance(cursor, dict):
                cursor = None
                break
            cursor = cursor.get(key)
        if isinstance(cursor, dict):
            for name, spec in cursor.items():
                collected[name] = normalize_dep(spec)
    return collected


def find_manifests(root: Path) -> list[Path]:
    manifests: list[Path] = []
    for path in root.rglob("Cargo.toml"):
        if any(part in SKIP_PARTS for part in path.parts):
            continue
        manifests.append(path)
    return sorted(manifests)


def render_manifest(path: Path, root: Path) -> str:
    data = load_toml(path)
    package = data.get("package", {})
    name = package.get("name", "<workspace>")
    version = package.get("version", "<none>")
    deps = collect_dependencies(data)
    tracked = {crate: deps[crate] for crate in TRACKED_CRATES if crate in deps}

    lines = [f"[{path.relative_to(root)}] {name} {version}"]
    if not tracked:
        lines.append("  tracked crates: none")
        return "\n".join(lines)

    for crate, spec in tracked.items():
        lines.append(f"  - {crate}: {spec}")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Inventory Rust workspace manifests for crates relevant to the rust-hexagonal-modern skill."
    )
    parser.add_argument("root", nargs="?", default=".", help="Workspace root to scan")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    manifests = find_manifests(root)
    if not manifests:
        raise SystemExit(f"No Cargo.toml files found under {root}")

    print(f"Workspace root: {root}")
    print(f"Cargo manifests: {len(manifests)}")
    print()
    for manifest in manifests:
        print(render_manifest(manifest, root))
        print()


if __name__ == "__main__":
    main()
