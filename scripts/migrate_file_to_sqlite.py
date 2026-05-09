#!/usr/bin/env python3
"""
Migrate Xavier memory-store.json data to SQLite.

Usage:
    python migrate_file_to_sqlite.py [--workspace WORKSPACE_ID] [--db-path /path/to/xavier.sqlite3]
    python migrate_file_to_sqlite.py --reinstall   # Recreate schema

Environment variables:
    XAVIER_MEMORY_SQLITE_PATH  - Path to SQLite database (default: ./data/workspaces/WORKSPACE/memory-store.sqlite3)
    XAVIER_WORKSPACE_DIR       - Path to workspaces directory (default: ./data/workspaces)

This script creates a SQLite database with the same schema as the file-based
memory store but with proper indexes for fast retrieval.
"""

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any

import sqlite3

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

WORKSPACE_DIR = os.environ.get("XAVIER_WORKSPACE_DIR", "./data/workspaces")

MEMORY_TABLE = "memory_records"
BELIEF_TABLE = "belief_states"
SESSION_TABLE = "session_tokens"
CHECKPOINT_TABLE = "checkpoint_records"

SCHEMA_SQL = f"""
-- Xavier Memory SQLite Schema
CREATE TABLE IF NOT EXISTS {MEMORY_TABLE} (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    path            TEXT NOT NULL,
    content         TEXT NOT NULL,
    metadata        TEXT NOT NULL DEFAULT '{{}}',
    embedding       TEXT NOT NULL DEFAULT '[]',
    created_at      TEXT,
    updated_at      TEXT,
    revision        INTEGER NOT NULL DEFAULT 1,
    primary         INTEGER NOT NULL DEFAULT 1,
    parent_id       TEXT,
    revisions       TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS {BELIEF_TABLE} (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    entity          TEXT NOT NULL,
    predicate       TEXT NOT NULL,
    value           TEXT NOT NULL,
    confidence      REAL NOT NULL DEFAULT 0.0,
    provenance      TEXT NOT NULL DEFAULT '{{}}',
    created_at      TEXT,
    updated_at      TEXT
);

CREATE TABLE IF NOT EXISTS {SESSION_TABLE} (
    token           TEXT PRIMARY KEY,
    created_at      TEXT,
    expires_at      TEXT
);

CREATE TABLE IF NOT EXISTS {CHECKPOINT_TABLE} (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    agent_id        TEXT,
    session_id      TEXT,
    checkpoint_type TEXT,
    state           TEXT NOT NULL DEFAULT '{{}}',
    created_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_memory_workspace ON {MEMORY_TABLE}(workspace_id);
CREATE INDEX IF NOT EXISTS idx_memory_path ON {MEMORY_TABLE}(path);
CREATE INDEX IF NOT EXISTS idx_memory_created ON {MEMORY_TABLE}(created_at);
CREATE INDEX IF NOT EXISTS idx_belief_workspace ON {BELIEF_TABLE}(workspace_id);
CREATE INDEX IF NOT EXISTS idx_belief_entity ON {BELIEF_TABLE}(entity);
CREATE INDEX IF NOT EXISTS idx_checkpoint_workspace ON {CHECKPOINT_TABLE}(workspace_id);
"""

INSERT_MEMORY_SQL = f"""
INSERT OR REPLACE INTO {MEMORY_TABLE}
    (id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary, parent_id, revisions)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
"""

INSERT_BELIEF_SQL = f"""
INSERT OR REPLACE INTO {BELIEF_TABLE}
    (id, workspace_id, entity, predicate, value, confidence, provenance, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
"""

INSERT_CHECKPOINT_SQL = f"""
INSERT OR REPLACE INTO {CHECKPOINT_TABLE}
    (id, workspace_id, agent_id, session_id, checkpoint_type, state, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?)
"""


@dataclass
class MigrationStats:
    memories_read: int = 0
    memories_written: int = 0
    beliefs_read: int = 0
    beliefs_written: int = 0
    checkpoints_written: int = 0
    errors: list = field(default_factory=list)

    def summary(self) -> str:
        return (
            f"Memories: {self.memories_written}/{self.memories_read}, "
            f"Beliefs: {self.beliefs_written}/{self.beliefs_read}, "
            f"Checkpoints: {self.checkpoints_written}, "
            f"Errors: {len(self.errors)}"
        )


# ---------------------------------------------------------------------------
# SQLite helpers
# ---------------------------------------------------------------------------

def get_connection(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def init_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(SCHEMA_SQL)
    conn.commit()


def recreate_schema(conn: sqlite3.Connection) -> None:
    """Drop all tables and recreate (--reinstall mode)."""
    cursor = conn.cursor()
    for table in [MEMORY_TABLE, BELIEF_TABLE, SESSION_TABLE, CHECKPOINT_TABLE]:
        try:
            cursor.execute(f"DROP TABLE IF EXISTS {table}")
        except Exception as e:
            print(f"  [warn] Could not drop {table}: {e}")
    conn.commit()
    init_schema(conn)


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

def insert_memories(conn: sqlite3.Connection, records: list[dict]) -> int:
    """Insert memory records. Returns count written."""
    cursor = conn.cursor()
    count = 0
    for rec in records:
        try:
            cursor.execute(
                INSERT_MEMORY_SQL,
                (
                    rec.get("id"),
                    rec.get("workspace_id", "default"),
                    rec.get("path", ""),
                    rec.get("content", ""),
                    json.dumps(rec.get("metadata", {})),
                    json.dumps(rec.get("embedding", [])),
                    rec.get("created_at"),
                    rec.get("updated_at"),
                    rec.get("revision", 1),
                    1 if rec.get("primary", True) else 0,
                    rec.get("parent_id"),
                    json.dumps(rec.get("revisions", [])),
                ),
            )
            count += 1
        except Exception as e:
            pass
    conn.commit()
    return count


def insert_beliefs(conn: sqlite3.Connection, beliefs: list[dict], workspace_id: str) -> int:
    """Insert belief records. Returns count written."""
    cursor = conn.cursor()
    count = 0
    for b in beliefs:
        try:
            cursor.execute(
                INSERT_BELIEF_SQL,
                (
                    b.get("id"),
                    workspace_id,
                    b.get("entity", ""),
                    b.get("predicate", ""),
                    b.get("value", ""),
                    b.get("confidence", 0.0),
                    json.dumps(b.get("provenance", {})),
                    b.get("created_at"),
                    b.get("updated_at"),
                ),
            )
            count += 1
        except Exception:
            pass
    conn.commit()
    return count


def insert_checkpoints(conn: sqlite3.Connection, checkpoints: list[dict], workspace_id: str) -> int:
    """Insert checkpoint records. Returns count written."""
    cursor = conn.cursor()
    count = 0
    for cp in checkpoints:
        try:
            cursor.execute(
                INSERT_CHECKPOINT_SQL,
                (
                    cp.get("id"),
                    workspace_id,
                    cp.get("agent_id"),
                    cp.get("session_id"),
                    cp.get("checkpoint_type"),
                    json.dumps(cp.get("state", {})),
                    cp.get("created_at"),
                ),
            )
            count += 1
        except Exception:
            pass
    conn.commit()
    return count


# ---------------------------------------------------------------------------
# Main migration
# ---------------------------------------------------------------------------

def migrate_workspace(workspace_id: str, db_path: str, reinstall: bool) -> MigrationStats:
    stats = MigrationStats()

    # Ensure parent dir
    os.makedirs(os.path.dirname(db_path), exist_ok=True)

    print(f"\nMigrating workspace '{workspace_id}' to SQLite at '{db_path}'...")
    conn = get_connection(db_path)

    if reinstall:
        print("  Recreating schema...")
        recreate_schema(conn)
    else:
        init_schema(conn)

    # Load file
    store = load_memory_store(WORKSPACE_DIR, workspace_id)
    if not store:
        print(f"  No data to migrate for workspace '{workspace_id}'.")
        conn.close()
        return stats

    workspace_data = store.get("workspaces", {}).get(workspace_id, {})
    memories = workspace_data.get("memories", [])
    beliefs = workspace_data.get("beliefs", [])
    checkpoints = workspace_data.get("checkpoints", [])

    stats.memories_read = len(memories)
    stats.beliefs_read = len(beliefs)

    print(f"  Found {stats.memories_read} memories, {stats.beliefs_read} beliefs, {len(checkpoints)} checkpoints")

    # Migrate
    print("  Inserting memories...")
    stats.memories_written = insert_memories(conn, memories)

    print("  Inserting beliefs...")
    stats.beliefs_written = insert_beliefs(conn, beliefs, workspace_id)

    print("  Inserting checkpoints...")
    stats.checkpoints_written = insert_checkpoints(conn, checkpoints, workspace_id)

    conn.close()
    return stats


def main():
    parser = argparse.ArgumentParser(description="Migrate memory-store.json to SQLite")
    parser.add_argument(
        "--workspace",
        default="default",
        help="Workspace ID to migrate (default: default)",
    )
    parser.add_argument(
        "--db-path",
        default=None,
        help="Path to SQLite database (default: <WORKSPACE_DIR>/<WORKSPACE>/memory-store.sqlite3)",
    )
    parser.add_argument(
        "--reinstall",
        action="store_true",
        help="Drop and recreate all tables before migrating",
    )
    parser.add_argument(
        "--all-workspaces",
        action="store_true",
        help="Migrate all workspaces found in WORKSPACE_DIR",
    )
    args = parser.parse_args()

    print("=" * 60)
    print("Xavier → SQLite Migration Tool")
    print("=" * 60)

    start = time.time()

    if args.all_workspaces:
        ws_path = WORKSPACE_DIR
        if os.path.isdir(ws_path):
            workspaces = [d for d in os.listdir(ws_path) if os.path.isdir(os.path.join(ws_path, d))]
        else:
            workspaces = ["default"]
    else:
        workspaces = [args.workspace]

    total_stats = MigrationStats()
    for ws in workspaces:
        if args.db_path and not args.all_workspaces:
            db_path = args.db_path
        else:
            db_path = os.path.join(WORKSPACE_DIR, ws, "memory-store.sqlite3")

        s = migrate_workspace(ws, db_path, args.reinstall)
        total_stats.memories_read += s.memories_read
        total_stats.memories_written += s.memories_written
        total_stats.beliefs_read += s.beliefs_read
        total_stats.beliefs_written += s.beliefs_written
        total_stats.checkpoints_written += s.checkpoints_written
        total_stats.errors.extend(s.errors)

    elapsed = time.time() - start
    print("\n" + "=" * 60)
    print("Migration Complete")
    print("=" * 60)
    print(f"Time elapsed  : {elapsed:.1f}s")
    print(f"Workspaces    : {len(workspaces)}")
    print(f"Memories      : {total_stats.memories_written}/{total_stats.memories_read}")
    print(f"Beliefs       : {total_stats.beliefs_written}/{total_stats.beliefs_read}")
    print(f"Checkpoints   : {total_stats.checkpoints_written}")
    if total_stats.errors:
        print(f"Errors        : {len(total_stats.errors)}")
        for e in total_stats.errors[:5]:
            print(f"  - {e}")
    else:
        print("Errors        : none")

    return 0


if __name__ == "__main__":
    sys.exit(main())