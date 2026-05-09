#!/usr/bin/env python3
"""
Migrate Xavier memory-store.json data to SurrealDB.

Usage:
    python migrate_file_to_surreal.py [--workspace WORKSPACE_ID] [--batch-size 100]
    python migrate_file_to_surreal.py --reinstall   # Drop and recreate tables first

Environment variables:
    XAVIER_SURREALDB_URL   - SurrealDB WebSocket URL (default: ws://localhost:8000)
    XAVIER_SURREALDB_USER  - Database user (default: root)
    XAVIER_SURREALDB_PASS  - Database password (default: root)
    XAVIER_SURREALDB_NS    - Namespace (default: xavier)
    XAVIER_SURREALDB_DB    - Database name (default: memory)
    XAVIER_WORKSPACE_DIR   - Path to workspaces directory (default: ./data/workspaces)

The script reads the memory-store.json file for each workspace and inserts
the records into SurrealDB using the REST API at http://localhost:8000/sql.
"""

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from typing import Any

import requests

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SURREALDB_URL = os.environ.get("XAVIER_SURREALDB_URL", "http://localhost:8000/sql")
SURREALDB_USER = os.environ.get("XAVIER_SURREALDB_USER", "root")
SURREALDB_PASS = os.environ.get("XAVIER_SURREALDB_PASS", "root")
SURREALDB_NS = os.environ.get("XAVIER_SURREALDB_NS", "xavier")
SURREALDB_DB = os.environ.get("XAVIER_SURREALDB_DB", "memory")
WORKSPACE_DIR = os.environ.get("XAVIER_WORKSPACE_DIR", "./data/workspaces")

MEMORY_TABLE = "memory_records"
BELIEF_TABLE = "belief_states"
SESSION_TABLE = "session_tokens"
CHECKPOINT_TABLE = "checkpoint_records"

HEADERS = {
    "Content-Type": "application/json",
    "NS": SURREALDB_NS,
    "DB": SURREALDB_DB,
}


@dataclass
class MigrationStats:
    memories_read: int = 0
    memories_written: int = 0
    beliefs_read: int = 0
    beliefs_written: int = 0
    errors: list = field(default_factory=list)

    def summary(self) -> str:
        return (
            f"Memories: {self.memories_written}/{self.memories_read} written, "
            f"Beliefs: {self.beliefs_written}/{self.beliefs_read} written, "
            f"Errors: {len(self.errors)}"
        )


# ---------------------------------------------------------------------------
# SurrealDB REST helpers
# ---------------------------------------------------------------------------

def surreal_query(sql: str, vars: dict | None = None) -> list[dict]:
    """Execute a SQL statement against SurrealDB via REST API."""
    body: dict[str, Any] = {"sql": sql}
    if vars:
        body["vars"] = vars
    resp = requests.post(SURREALDB_URL, headers=HEADERS, json=body, auth=(SURREALDB_USER, SURREALDB_PASS))
    resp.raise_for_status()
    result = resp.json()
    if isinstance(result, dict) and result.get("code") == 401:
        raise RuntimeError(f"SurrealDB auth failed: {result}")
    return result if isinstance(result, list) else [result]


def surreal_mutation(sql: str, vars: dict | None = None) -> tuple[int, list]:
    """Execute a mutating SQL statement and return (rows affected, results)."""
    body: dict[str, Any] = {"sql": sql}
    if vars:
        body["vars"] = vars
    resp = requests.post(
        SURREALDB_URL,
        headers=HEADERS,
        json=body,
        auth=(SURREALDB_USER, SURREALDB_PASS),
    )
    resp.raise_for_status()
    result = resp.json()
    if isinstance(result, dict) and result.get("code") == 401:
        raise RuntimeError(f"SurrealDB auth failed: {result}")
    # SurrealDB REST returns a list; first item has status info
    if isinstance(result, list) and result:
        first = result[0]
        if isinstance(first, dict):
            rows = first.get("num_affect_rows", 0)
            return rows, first.get("result", result)
    return 0, result


def ensure_namespace_and_db() -> None:
    """Create the namespace + database if they don't exist."""
    # Use root endpoint to create NS/DB
    root_headers = {"Content-Type": "application/json"}
    # Create namespace
    try:
        requests.post(
            f"http://localhost:8000/sql",
            headers=root_headers,
            json={"sql": f"CREATE NAMESPACE {SURREALDB_NS} IF NOT EXISTS"},
            auth=(SURREALDB_USER, SURREALDB_PASS),
        )
        print(f"  Namespace '{SURREALDB_NS}' ensured.")
    except Exception as e:
        print(f"  [warn] Could not create namespace (may already exist): {e}")

    # Create database inside namespace
    try:
        requests.post(
            f"http://localhost:8000/sql",
            headers=root_headers,
            json={"sql": f"USE NAMESPACE {SURREALDB_NS}; CREATE DATABASE {SURREALDB_DB} IF NOT EXISTS"},
            auth=(SURREALDB_USER, SURREALDB_PASS),
        )
        print(f"  Database '{SURREALDB_DB}' ensured.")
    except Exception as e:
        print(f"  [warn] Could not create database (may already exist): {e}")


def setup_tables() -> None:
    """Create tables and define schema."""
    print("Setting up SurrealDB schema...")
    ensure_namespace_and_db()

    # memory_records
    surreal_query(f"""
        DEFINE TABLE {MEMORY_TABLE} SCHEMAFULL PERMISSIONS FULL;
        DEFINE FIELD id ON {MEMORY_TABLE} TYPE string;
        DEFINE FIELD workspace_id ON {MEMORY_TABLE} TYPE string;
        DEFINE FIELD path ON {MEMORY_TABLE} TYPE string;
        DEFINE FIELD content ON {MEMORY_TABLE} TYPE string;
        DEFINE FIELD metadata ON {MEMORY_TABLE} TYPE object;
        DEFINE FIELD embedding ON {MEMORY_TABLE} TYPE array<float>;
        DEFINE FIELD created_at ON {MEMORY_TABLE} TYPE datetime;
        DEFINE FIELD updated_at ON {MEMORY_TABLE} TYPE datetime;
        DEFINE FIELD revision ON {MEMORY_TABLE} TYPE int;
        DEFINE FIELD primary ON {MEMORY_TABLE} TYPE bool;
        DEFINE FIELD parent_id ON {MEMORY_TABLE} TYPE option<string>;
        DEFINE FIELD revisions ON {MEMORY_TABLE} TYPE array;
        DEFINE INDEX idx_memory_workspace ON {MEMORY_TABLE} COLUMNS workspace_id;
        DEFINE INDEX idx_memory_path ON {MEMORY_TABLE} COLUMNS path;
    """)

    # belief_states
    surreal_query(f"""
        DEFINE TABLE {BELIEF_TABLE} SCHEMAFULL PERMISSIONS FULL;
        DEFINE FIELD id ON {BELIEF_TABLE} TYPE string;
        DEFINE FIELD workspace_id ON {BELIEF_TABLE} TYPE string;
        DEFINE FIELD entity ON {BELIEF_TABLE} TYPE string;
        DEFINE FIELD predicate ON {BELIEF_TABLE} TYPE string;
        DEFINE FIELD value ON {BELIEF_TABLE} TYPE string;
        DEFINE FIELD confidence ON {BELIEF_TABLE} TYPE float;
        DEFINE FIELD provenance ON {BELIEF_TABLE} TYPE object;
        DEFINE FIELD created_at ON {BELIEF_TABLE} TYPE datetime;
        DEFINE FIELD updated_at ON {BELIEF_TABLE} TYPE datetime;
    """)

    # session_tokens
    surreal_query(f"""
        DEFINE TABLE {SESSION_TABLE} SCHEMAFULL PERMISSIONS FULL;
        DEFINE FIELD token ON {SESSION_TABLE} TYPE string;
        DEFINE FIELD created_at ON {SESSION_TABLE} TYPE datetime;
        DEFINE FIELD expires_at ON {SESSION_TABLE} TYPE datetime;
    """)

    # checkpoint_records
    surreal_query(f"""
        DEFINE TABLE {CHECKPOINT_TABLE} SCHEMAFULL PERMISSIONS FULL;
        DEFINE FIELD id ON {CHECKPOINT_TABLE} TYPE string;
        DEFINE FIELD workspace_id ON {CHECKPOINT_TABLE} TYPE string;
        DEFINE FIELD agent_id ON {CHECKPOINT_TABLE} TYPE string;
        DEFINE FIELD session_id ON {CHECKPOINT_TABLE} TYPE string;
        DEFINE FIELD checkpoint_type ON {CHECKPOINT_TABLE} TYPE string;
        DEFINE FIELD state ON {CHECKPOINT_TABLE} TYPE object;
        DEFINE FIELD created_at ON {CHECKPOINT_TABLE} TYPE datetime;
    """)

    print("  Schema created.")


def drop_and_recreate_tables() -> None:
    """Drop all tables and recreate them (--reinstall mode)."""
    print("Dropping existing tables...")
    for table in [MEMORY_TABLE, BELIEF_TABLE, SESSION_TABLE, CHECKPOINT_TABLE]:
        try:
            surreal_query(f"DROP TABLE {table};")
            print(f"  Dropped {table}.")
        except Exception as e:
            print(f"  [warn] Could not drop {table}: {e}")
    setup_tables()


# ---------------------------------------------------------------------------
# JSON loading
# ---------------------------------------------------------------------------

def load_memory_store(workspace_dir: str, workspace_id: str) -> dict | None:
    """Load memory-store.json for a workspace."""
    path = os.path.join(workspace_dir, workspace_id, "memory-store.json")
    if not os.path.exists(path):
        print(f"  [warn] No memory-store.json at {path}")
        return None
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Insert helpers
# ---------------------------------------------------------------------------

def insert_memory_record(record: dict) -> bool:
    """Insert a single memory record via SurrealDB REST."""
    sql = f"""
        INSERT INTO {MEMORY_TABLE} {{
            id: $id,
            workspace_id: $workspace_id,
            path: $path,
            content: $content,
            metadata: $metadata,
            embedding: $embedding,
            created_at: $created_at,
            updated_at: $updated_at,
            revision: $revision,
            primary: $primary,
            parent_id: $parent_id,
            revisions: $revisions
        }}
    """
    vars = {
        "id": record.get("id"),
        "workspace_id": record.get("workspace_id", "default"),
        "path": record.get("path", ""),
        "content": record.get("content", ""),
        "metadata": record.get("metadata", {}),
        "embedding": record.get("embedding", []),
        "created_at": record.get("created_at"),
        "updated_at": record.get("updated_at"),
        "revision": record.get("revision", 1),
        "primary": record.get("primary", True),
        "parent_id": record.get("parent_id"),
        "revisions": record.get("revisions", []),
    }
    try:
        surreal_mutation(sql, vars)
        return True
    except Exception as e:
        return False


def insert_belief_records(beliefs: list, workspace_id: str) -> int:
    """Insert belief records. Returns count of successful inserts."""
    count = 0
    for belief in beliefs:
        sql = f"""
            INSERT INTO {BELIEF_TABLE} {{
                id: $id,
                workspace_id: $workspace_id,
                entity: $entity,
                predicate: $predicate,
                value: $value,
                confidence: $confidence,
                provenance: $provenance,
                created_at: $created_at,
                updated_at: $updated_at
            }}
        """
        vars = {
            "id": belief.get("id"),
            "workspace_id": workspace_id,
            "entity": belief.get("entity", ""),
            "predicate": belief.get("predicate", ""),
            "value": belief.get("value", ""),
            "confidence": belief.get("confidence", 0.0),
            "provenance": belief.get("provenance", {}),
            "created_at": belief.get("created_at"),
            "updated_at": belief.get("updated_at"),
        }
        try:
            surreal_mutation(sql, vars)
            count += 1
        except Exception:
            pass
    return count


def insert_checkpoint_records(checkpoints: list, workspace_id: str) -> int:
    """Insert checkpoint records. Returns count of successful inserts."""
    count = 0
    for cp in checkpoints:
        sql = f"""
            INSERT INTO {CHECKPOINT_TABLE} {{
                id: $id,
                workspace_id: $workspace_id,
                agent_id: $agent_id,
                session_id: $session_id,
                checkpoint_type: $checkpoint_type,
                state: $state,
                created_at: $created_at
            }}
        """
        vars = {
            "id": cp.get("id"),
            "workspace_id": workspace_id,
            "agent_id": cp.get("agent_id"),
            "session_id": cp.get("session_id"),
            "checkpoint_type": cp.get("checkpoint_type"),
            "state": cp.get("state", {}),
            "created_at": cp.get("created_at"),
        }
        try:
            surreal_mutation(sql, vars)
            count += 1
        except Exception:
            pass
    return count


# ---------------------------------------------------------------------------
# Batch helpers
# ---------------------------------------------------------------------------

def batch_insert_memories(records: list[dict], batch_size: int = 100) -> tuple[int, list]:
    """
    Insert memory records in batches using a single multi-row INSERT.
    Returns (successful, errors).
    """
    written = 0
    errors = []
    for i in range(0, len(records), batch_size):
        batch = records[i : i + batch_size]
        # Build multi-value INSERT
        values = []
        for rec in batch:
            meta = rec.get("metadata", {})
            values.append(
                f"('{rec.get('id', '')}', '{rec.get('workspace_id', 'default')}', "
                f"'{_escape_str(rec.get('path', ''))}', "
                f"'{_escape_str(rec.get('content', ''))}', "
                f"{json.dumps(meta)}, "
                f"{json.dumps(rec.get('embedding', []))}, "
                f"'{rec.get('created_at', '')}', '{rec.get('updated_at', '')}', "
                f"{rec.get('revision', 1)}, {rec.get('primary', True)}, "
                f"{'NULL' if rec.get('parent_id') is None else f\"'{rec.get('parent_id')}'\"}, "
                f"{json.dumps(rec.get('revisions', []))})"
            )

        sql = f"INSERT INTO {MEMORY_TABLE} (id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary, parent_id, revisions) VALUES {', '.join(values)}"

        try:
            rows, _ = surreal_mutation(sql)
            written += rows
        except Exception as e:
            errors.append(str(e))
            # Fallback to individual inserts
            for rec in batch:
                if insert_memory_record(rec):
                    written += 1

    return written, errors


def _escape_str(s: str) -> str:
    """Escape single quotes in a string for SQL."""
    return s.replace("\\", "\\\\").replace("'", "\\'")


# ---------------------------------------------------------------------------
# Main migration
# ---------------------------------------------------------------------------

def migrate_workspace(workspace_id: str, batch_size: int, reinstall: bool) -> MigrationStats:
    stats = MigrationStats()

    print(f"\nMigrating workspace '{workspace_id}'...")

    # Load file
    store = load_memory_store(WORKSPACE_DIR, workspace_id)
    if not store:
        print(f"  No data to migrate for workspace '{workspace_id}'.")
        return stats

    workspace_data = store.get("workspaces", {}).get(workspace_id, {})
    memories = workspace_data.get("memories", [])
    beliefs = workspace_data.get("beliefs", [])
    checkpoints = workspace_data.get("checkpoints", [])

    stats.memories_read = len(memories)
    stats.beliefs_read = len(beliefs)

    print(f"  Found {stats.memories_read} memories, {stats.beliefs_read} beliefs, {len(checkpoints)} checkpoints")

    if reinstall:
        drop_and_recreate_tables()

    # Migrate memories
    print(f"  Inserting memories in batches of {batch_size}...")
    written, errors = batch_insert_memories(memories, batch_size)
    stats.memories_written = written
    stats.errors.extend(errors)

    # Migrate beliefs
    print(f"  Inserting beliefs...")
    stats.beliefs_written = insert_belief_records(beliefs, workspace_id)

    # Migrate checkpoints
    print(f"  Inserting checkpoints...")
    cp_written = insert_checkpoint_records(checkpoints, workspace_id)
    print(f"  Checkpoints written: {cp_written}")

    return stats


def main():
    parser = argparse.ArgumentParser(description="Migrate memory-store.json to SurrealDB")
    parser.add_argument(
        "--workspace",
        default="default",
        help="Workspace ID to migrate (default: default)",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=100,
        help="Batch insert size (default: 100)",
    )
    parser.add_argument(
        "--reinstall",
        action="store_true",
        help="Drop and recreate tables before migrating",
    )
    parser.add_argument(
        "--all-workspaces",
        action="store_true",
        help="Migrate all workspaces found in WORKSPACE_DIR",
    )
    args = parser.parse_args()

    print("=" * 60)
    print("Xavier → SurrealDB Migration Tool")
    print("=" * 60)
    print(f"SurrealDB URL : {SURREALDB_URL}")
    print(f"Namespace     : {SURREALDB_NS}")
    print(f"Database      : {SURREALDB_DB}")
    print(f"Workspace dir : {WORKSPACE_DIR}")

    if args.reinstall:
        drop_and_recreate_tables()
    else:
        ensure_namespace_and_db()

    start = time.time()

    if args.all_workspaces:
        # Find all workspace directories
        ws_path = WORKSPACE_DIR
        if os.path.isdir(ws_path):
            workspaces = [
                d
                for d in os.listdir(ws_path)
                if os.path.isdir(os.path.join(ws_path, d))
            ]
        else:
            workspaces = ["default"]
    else:
        workspaces = [args.workspace]

    total_stats = MigrationStats()
    for ws in workspaces:
        s = migrate_workspace(ws, args.batch_size, args.reinstall and ws == workspaces[0])
        total_stats.memories_read += s.memories_read
        total_stats.memories_written += s.memories_written
        total_stats.beliefs_read += s.beliefs_read
        total_stats.beliefs_written += s.beliefs_written
        total_stats.errors.extend(s.errors)
        if args.reinstall and ws == workspaces[0]:
            pass  # already reinstalled

    elapsed = time.time() - start
    print("\n" + "=" * 60)
    print("Migration Complete")
    print("=" * 60)
    print(f"Time elapsed  : {elapsed:.1f}s")
    print(f"Workspaces    : {len(workspaces)}")
    print(f"Memories      : {total_stats.memories_written}/{total_stats.memories_read} written")
    print(f"Beliefs       : {total_stats.beliefs_written}/{total_stats.beliefs_read} written")
    if total_stats.errors:
        print(f"Errors        : {len(total_stats.errors)}")
        for e in total_stats.errors[:5]:
            print(f"  - {e}")
    else:
        print("Errors        : none")

    return 0


if __name__ == "__main__":
    sys.exit(main())