#!/usr/bin/env python3
"""
Xavier CLI - Client for Xavier Memory Server

Usage:
    python xavier_cli.py create --path "project/doc" --content "Content here"
    python xavier_cli.py search --query "authentication"
    python xavier_cli.py delete --path "project/doc"
    python xavier_cli.py reset
    python xavier_cli.py query --query "What changed?"
    python xavier_cli.py code-find --query "AgentRuntime"
    python xavier_cli.py code-stats
    python xavier_cli.py sync-gitcore --project "E:/scripts-python/manteniapp"
"""

import argparse
import json
import os
from pathlib import Path

import requests

# Configuration
XAVIER_URL = os.environ.get("XAVIER_URL", "http://localhost:8003")


def get_required_xavier_token() -> str:
    for env_var in ("XAVIER_TOKEN", "XAVIER_API_KEY", "XAVIER_TOKEN"):
        token = os.environ.get(env_var, "").strip()
        if token:
            return token
    raise RuntimeError("Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.")


XAVIER_TOKEN = get_required_xavier_token()


class XavierClient:
    """Client for the Xavier HTTP API."""

    def __init__(self, url=XAVIER_URL, token=XAVIER_TOKEN):
        self.url = url.rstrip("/")
        self.token = token
        self.headers = {"X-Xavier-Token": token}

    def create_memory(self, path: str, content: str, metadata: dict = None):
        response = requests.post(
            f"{self.url}/memory/add",
            json={
                "path": path,
                "content": content,
                "metadata": metadata or {},
            },
            headers=self.headers,
        )
        return response.json()

    def search_memory(self, query: str, limit: int = 10):
        response = requests.post(
            f"{self.url}/memory/search",
            json={"query": query, "limit": limit},
            headers=self.headers,
        )
        return response.json()

    def delete_memory(self, mem_id: str = None, path: str = None):
        payload = {}
        if mem_id:
            payload["id"] = mem_id
        if path:
            payload["path"] = path
        response = requests.post(
            f"{self.url}/memory/delete",
            json=payload,
            headers=self.headers,
        )
        return response.json()

    def reset_memory(self):
        response = requests.post(
            f"{self.url}/memory/reset",
            headers=self.headers,
        )
        return response.json()

    def query_memory(self, query: str, limit: int = 10):
        response = requests.post(
            f"{self.url}/memory/query",
            json={"query": query, "limit": limit},
            headers=self.headers,
        )
        return response.json()

    def code_find(self, query: str, limit: int = 10, kind: str = None):
        payload = {"query": query, "limit": limit}
        if kind:
            payload["kind"] = kind
        response = requests.post(
            f"{self.url}/code/find",
            json=payload,
            headers=self.headers,
        )
        return response.json()

    def code_stats(self):
        response = requests.get(
            f"{self.url}/code/stats",
            headers=self.headers,
        )
        return response.json()

    def sync_gitcore_project(self, project_path: str):
        project_path = Path(project_path)

        if not project_path.exists():
            return {"error": f"Project not found: {project_path}"}

        docs_path = project_path / "DOCS" / "SRC"
        if not docs_path.exists():
            docs_path = project_path / "docs" / "src"

        if not docs_path.exists():
            return {"error": f"DOCS/SRC not found in {project_path}"}

        synced = 0
        errors = []

        for md_file in docs_path.rglob("*.md"):
            if md_file.name.startswith("."):
                continue

            try:
                content = md_file.read_text(encoding="utf-8")
                path = f"{project_path.name}/{md_file.relative_to(docs_path)}"
                metadata = {
                    "project": project_path.name,
                    "type": "src",
                    "file": str(md_file.relative_to(project_path)),
                }
                self.create_memory(path, content, metadata)
                synced += 1
                print(f"  [OK] {path}")
            except Exception as exc:
                errors.append(f"{md_file}: {exc}")
                print(f"  [ERROR] {md_file}: {exc}")

        return {"synced": synced, "errors": errors, "project": project_path.name}


def main():
    parser = argparse.ArgumentParser(description="Xavier CLI")
    subparsers = parser.add_subparsers(dest="command", help="Commands")

    create_parser = subparsers.add_parser("create", help="Create memory")
    create_parser.add_argument("--path", required=True, help="Memory path")
    create_parser.add_argument("--content", required=True, help="Memory content")
    create_parser.add_argument("--metadata", help="Metadata JSON")

    search_parser = subparsers.add_parser("search", help="Search memories")
    search_parser.add_argument("--query", required=True, help="Search query")
    search_parser.add_argument("--limit", type=int, default=10, help="Limit results")

    delete_parser = subparsers.add_parser("delete", help="Delete memory by id or path")
    delete_parser.add_argument("--id", help="Memory ID")
    delete_parser.add_argument("--path", help="Memory path")

    subparsers.add_parser("reset", help="Reset in-memory documents")

    query_parser = subparsers.add_parser("query", help="Query the runtime")
    query_parser.add_argument("--query", required=True, help="Runtime query")
    query_parser.add_argument("--limit", type=int, default=10, help="Limit results")

    code_find_parser = subparsers.add_parser("code-find", help="Search indexed code")
    code_find_parser.add_argument("--query", required=True, help="Search query")
    code_find_parser.add_argument("--limit", type=int, default=10, help="Limit results")
    code_find_parser.add_argument("--kind", help="Optional symbol kind filter")

    subparsers.add_parser("code-stats", help="Get code index stats")

    sync_parser = subparsers.add_parser("sync-gitcore", help="Sync GitCore project")
    sync_parser.add_argument("--project", required=True, help="Project path")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    client = XavierClient()

    if args.command == "create":
        metadata = json.loads(args.metadata) if args.metadata else None
        result = client.create_memory(args.path, args.content, metadata)
        print(json.dumps(result, indent=2))
    elif args.command == "search":
        result = client.search_memory(args.query, args.limit)
        print(json.dumps(result, indent=2))
    elif args.command == "delete":
        result = client.delete_memory(args.id, args.path)
        print(json.dumps(result, indent=2))
    elif args.command == "reset":
        result = client.reset_memory()
        print(json.dumps(result, indent=2))
    elif args.command == "query":
        result = client.query_memory(args.query, args.limit)
        print(json.dumps(result, indent=2))
    elif args.command == "code-find":
        result = client.code_find(args.query, args.limit, args.kind)
        print(json.dumps(result, indent=2))
    elif args.command == "code-stats":
        result = client.code_stats()
        print(json.dumps(result, indent=2))
    elif args.command == "sync-gitcore":
        print(f"Syncing GitCore project: {args.project}")
        result = client.sync_gitcore_project(args.project)
        print(f"\nResult: {json.dumps(result, indent=2)}")


if __name__ == "__main__":
    main()
