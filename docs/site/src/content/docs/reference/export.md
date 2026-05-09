---
title: Public Export
description: Export Xavier public data manifests to GitHub and analytical artifacts to Hugging Face.
---

# Public Export

`xavier export --public` publishes a Xavier dataset as a two-host public release:

- GitHub stores the lightweight context layer as NDJSON manifests and schemas.
- Hugging Face stores the heavy analytical and vector artifacts.

The protocol lets agents inspect public context from GitHub raw URLs first, then fetch larger files from Hugging Face only when they need embeddings, metrics, full database snapshots, or vector indexes.

## Command

```bash
xavier export --public \
  --huggingface-repo iberi22/xavier-dataset \
  --huggingface-token $HUGGINGFACE_TOKEN
```

## Purpose

Public export is designed for portable, agent-readable Xavier releases. The GitHub repository keeps the files that are useful for discovery, validation, and quick context loading. The Hugging Face dataset stores larger files that are better served through dataset hosting and versioned artifact downloads.

Any agent with a Xavier node can understand this protocol:

1. Read the GitHub raw manifest.
2. Validate records against the included schemas.
3. Follow the Hugging Face URLs embedded in the NDJSON records for heavy artifacts.

## Publishing Pipeline

`xavier export --public` performs the public release as a split pipeline:

1. Generate NDJSON manifests, context records, and JSON schemas.
2. Commit the NDJSON and schema files to the GitHub dataset repository.
3. Generate Parquet files for embeddings and metrics.
4. Generate a complete `.sqlite3` database snapshot.
5. Generate vector artifacts such as `.lance/` or `.faiss`.
6. Upload Parquet, SQLite, Lance, and FAISS artifacts to Hugging Face.
7. Write Hugging Face artifact URLs into the NDJSON records so lightweight consumers can discover the heavy data layer.

## Flags

| Flag | Required | Description |
| --- | --- | --- |
| `--public` | Yes | Enables the public export layout and writes shareable artifact references. |
| `--huggingface-repo <owner/name>` | Yes | Hugging Face dataset repository that receives Parquet, SQLite, Lance, and FAISS artifacts. |
| `--huggingface-token <token>` | Yes | Token used to upload files to Hugging Face. Prefer passing it from an environment variable. |

## Repositories

The canonical public dataset target is:

- GitHub: `iberi22/xavier-dataset`
- Hugging Face: `iberi22/xavier-dataset`

The same owner/name is intentional. GitHub is the stable lightweight index, while Hugging Face is the large artifact store.

## Data Layers

| Layer | Location | Contents | Typical size |
| --- | --- | --- | --- |
| Manifest + context | GitHub raw | NDJSONs, schemas | ~1-10 MB |
| Analytical data | Hugging Face | Parquet files for embeddings and metrics | ~50-500 MB |
| Database + vectors | Hugging Face | `.sqlite3`, `.lance/`, `.faiss` | ~100 MB-2 GB |

## Output Format

The export produces a small GitHub-facing package and a larger Hugging Face package.

GitHub receives:

- NDJSON manifests.
- JSON schemas for validating records.
- References to the matching Hugging Face dataset artifacts.

Hugging Face receives:

- Parquet files for embeddings and metrics.
- A complete `.sqlite3` database snapshot.
- Vector stores such as `.lance/` or `.faiss`.

NDJSON records should include URLs for any heavy files stored in Hugging Face. Consumers can use GitHub raw for quick reads and Hugging Face for bulk downloads.

## Access Pattern

Use GitHub raw when an agent needs a small amount of public context, usually in the `~1-10 MB` range:

- dataset manifest
- record indexes
- schema definitions
- lightweight context records

Use Hugging Face when a workflow needs heavy artifacts, usually in the `~50 MB-2 GB` range:

- embedding tables
- metrics tables
- complete SQLite snapshots
- Lance or FAISS vector indexes

## Examples

Export to the canonical Xavier public dataset:

```bash
xavier export --public \
  --huggingface-repo iberi22/xavier-dataset \
  --huggingface-token $HUGGINGFACE_TOKEN
```

Use the GitHub raw layer when an agent needs lightweight context:

```text
https://raw.githubusercontent.com/iberi22/xavier-dataset/main/manifest.ndjson
```

Use the Hugging Face dataset when a workflow needs heavy analytical files:

```text
https://huggingface.co/datasets/iberi22/xavier-dataset
```

## Publishing Protocol

1. Generate NDJSON manifests and schemas.
2. Commit the NDJSON and schema files to `iberi22/xavier-dataset` on GitHub.
3. Generate Parquet, SQLite, Lance, and FAISS artifacts.
4. Upload heavy artifacts to the `iberi22/xavier-dataset` Hugging Face dataset.
5. Ensure NDJSON records point to their matching Hugging Face artifact URLs.

This split keeps public context cheap to inspect while preserving full analytical fidelity for downstream agents and notebooks.
