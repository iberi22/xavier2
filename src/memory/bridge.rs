use std::path::Path;
use walkdir::WalkDir;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs;

use crate::memory::{
    qmd_memory::QmdMemory,
    schema::{EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, TypedMemoryPayload},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSource {
    OpenclawMarkdown,
    EngramExport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BridgeImportOptions {
    pub project: Option<String>,
    pub scope: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BridgeImportStats {
    pub source: String,
    pub imported: usize,
    pub skipped: usize,
}

pub async fn import_from_path(
    memory: &QmdMemory,
    source: BridgeSource,
    path: impl AsRef<Path>,
    options: BridgeImportOptions,
) -> Result<BridgeImportStats> {
    match source {
        BridgeSource::OpenclawMarkdown => {
            import_openclaw_markdown_dir(memory, path.as_ref(), options).await
        }
        BridgeSource::EngramExport => {
            import_engram_export_file(memory, path.as_ref(), options).await
        }
    }
}

pub async fn import_openclaw_markdown_dir(
    memory: &QmdMemory,
    root: &Path,
    options: BridgeImportOptions,
) -> Result<BridgeImportStats> {
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }

        let raw = fs::read_to_string(path).await?;
        let (frontmatter, content) = split_frontmatter(&raw);
        let memory_type = frontmatter
            .get("memory_type")
            .and_then(|value| value.as_str())
            .unwrap_or("memory");
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let project = options
            .project
            .clone()
            .or_else(|| {
                frontmatter
                    .get("project")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .or_else(|| {
                frontmatter
                    .get("projects")
                    .and_then(|value| value.as_array())
                    .and_then(|value| value.first())
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            });

        let namespace = MemoryNamespace {
            agent_id: options.agent_id.clone(),
            session_id: options.session_id.clone(),
            project,
            scope: options.scope.clone(),
            ..MemoryNamespace::default()
        };
        let provenance = MemoryProvenance {
            source_app: Some("openclaw".to_string()),
            source_type: Some("markdown_memory".to_string()),
            file_path: Some(relative_path.clone()),
            observed_at: frontmatter
                .get("date")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            recorded_at: frontmatter
                .get("date")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            ..MemoryProvenance::default()
        };
        let typed = TypedMemoryPayload {
            kind: Some(map_openclaw_kind(memory_type)),
            evidence_kind: Some(EvidenceKind::Observation),
            namespace: Some(namespace),
            provenance: Some(provenance),
            ..Default::default()
        };

        let title = frontmatter
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or(relative_path.as_str());
        let document_path = format!("bridge/openclaw/{relative_path}");
        let metadata = json!({
            "title": title,
            "memory_type": memory_type,
            "projects": frontmatter.get("projects").cloned().unwrap_or(Value::Null),
            "tags": frontmatter.get("tags").cloned().unwrap_or(Value::Null),
        });

        if content.trim().is_empty() {
            skipped += 1;
            continue;
        }

        memory
            .add_document_typed(document_path, content, metadata, Some(typed))
            .await?;
        imported += 1;
    }

    Ok(BridgeImportStats {
        source: "openclaw_markdown".to_string(),
        imported,
        skipped,
    })
}

pub async fn import_engram_export_file(
    memory: &QmdMemory,
    path: &Path,
    options: BridgeImportOptions,
) -> Result<BridgeImportStats> {
    let payload = fs::read_to_string(path).await?;
    let export: Value = serde_json::from_str(&payload)?;
    import_engram_export(memory, &export, options).await
}

pub async fn import_engram_export(
    memory: &QmdMemory,
    export: &Value,
    options: BridgeImportOptions,
) -> Result<BridgeImportStats> {
    let mut imported = 0usize;

    for session in export
        .get("sessions")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let session_id = session
            .get("id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("engram session missing id"))?;
        let project = options.project.clone().or_else(|| {
            session
                .get("project")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
        let namespace = MemoryNamespace {
            agent_id: options.agent_id.clone(),
            session_id: Some(session_id.to_string()),
            project,
            scope: options.scope.clone(),
            ..MemoryNamespace::default()
        };
        let provenance = MemoryProvenance {
            source_app: Some("engram".to_string()),
            source_type: Some("session".to_string()),
            recorded_at: session
                .get("started_at")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            ..MemoryProvenance::default()
        };
        let content = format!(
            "Engram session {} for project {} in {}. Summary: {}",
            session_id,
            session
                .get("project")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            session
                .get("directory")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            session
                .get("summary")
                .and_then(|value| value.as_str())
                .unwrap_or("")
        );
        memory
            .add_document_typed(
                format!("bridge/engram/sessions/{session_id}"),
                content,
                json!({
                    "status": session.get("status").cloned().unwrap_or(Value::Null),
                }),
                Some(TypedMemoryPayload {
                    kind: Some(MemoryKind::Session),
                    evidence_kind: Some(EvidenceKind::SessionSummary),
                    namespace: Some(namespace),
                    provenance: Some(provenance),
                    ..Default::default()
                }),
            )
            .await?;
        imported += 1;
    }

    for observation in export
        .get("observations")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let observation_id = observation
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| anyhow!("engram observation missing id"))?;
        let session_id = observation
            .get("session_id")
            .and_then(|value| value.as_str())
            .or(options.session_id.as_deref())
            .ok_or_else(|| anyhow!("engram observation missing session_id"))?;
        let project = options.project.clone().or_else(|| {
            observation
                .get("project")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
        let scope = options.scope.clone().or_else(|| {
            observation
                .get("scope")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
        let observation_type = observation
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("discovery");

        let namespace = MemoryNamespace {
            agent_id: options.agent_id.clone(),
            session_id: Some(session_id.to_string()),
            project,
            scope,
            ..MemoryNamespace::default()
        };
        let provenance = MemoryProvenance {
            source_app: Some("engram".to_string()),
            source_type: Some("observation".to_string()),
            topic_key: observation
                .get("topic_key")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            tool_name: observation
                .get("tool_name")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            created_by: observation
                .get("created_by")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            observed_at: observation
                .get("created_at")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            recorded_at: observation
                .get("updated_at")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| {
                    observation
                        .get("created_at")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                }),
            ..MemoryProvenance::default()
        };
        let content = format!(
            "{}\n\n{}",
            observation
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("Observation"),
            observation
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
        );
        memory
            .add_document_typed(
                format!("bridge/engram/observations/{observation_id}"),
                content,
                json!({
                    "engram_type": observation_type,
                    "duplicate_count": observation.get("duplicate_count").cloned().unwrap_or(Value::Null),
                }),
                Some(TypedMemoryPayload {
                    kind: Some(map_engram_observation_kind(observation_type)),
                    evidence_kind: Some(EvidenceKind::Observation),
                    namespace: Some(namespace),
                    provenance: Some(provenance),
                    ..Default::default()
                }),
            )
            .await?;
        imported += 1;
    }

    for prompt in export
        .get("user_prompts")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let prompt_id = prompt
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| anyhow!("engram prompt missing id"))?;
        let session_id = prompt
            .get("session_id")
            .and_then(|value| value.as_str())
            .or(options.session_id.as_deref())
            .ok_or_else(|| anyhow!("engram prompt missing session_id"))?;
        let namespace = MemoryNamespace {
            agent_id: options.agent_id.clone(),
            session_id: Some(session_id.to_string()),
            project: options.project.clone().or_else(|| {
                prompt
                    .get("project")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            }),
            scope: options.scope.clone(),
            ..MemoryNamespace::default()
        };
        let provenance = MemoryProvenance {
            source_app: Some("engram".to_string()),
            source_type: Some("user_prompt".to_string()),
            recorded_at: prompt
                .get("created_at")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            ..MemoryProvenance::default()
        };
        let content = prompt
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        memory
            .add_document_typed(
                format!("bridge/engram/prompts/{prompt_id}"),
                content,
                json!({}),
                Some(TypedMemoryPayload {
                    kind: Some(MemoryKind::Session),
                    evidence_kind: Some(EvidenceKind::UserPrompt),
                    namespace: Some(namespace),
                    provenance: Some(provenance),
                    ..Default::default()
                }),
            )
            .await?;
        imported += 1;
    }

    Ok(BridgeImportStats {
        source: "engram_export".to_string(),
        imported,
        skipped: 0,
    })
}

fn split_frontmatter(raw: &str) -> (Value, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (json!({}), raw.to_string());
    }

    let mut lines = trimmed.lines();
    let _ = lines.next();
    let mut frontmatter_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_frontmatter = true;

    for line in lines {
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            continue;
        }
        if in_frontmatter {
            frontmatter_lines.push(line.to_string());
        } else {
            body_lines.push(line.to_string());
        }
    }

    let mut metadata = serde_json::Map::new();
    for line in frontmatter_lines {
        if let Some((key, value)) = line.split_once(':') {
            metadata.insert(
                key.trim().to_string(),
                parse_frontmatter_value(value.trim().trim_matches('"')),
            );
        }
    }

    (
        Value::Object(metadata),
        body_lines.join("\n").trim().to_string(),
    )
}

fn parse_frontmatter_value(value: &str) -> Value {
    if value.starts_with('[') && value.ends_with(']') {
        let items = value
            .trim_matches(['[', ']'])
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(value.trim_matches('"').to_string()))
            .collect::<Vec<_>>();
        return Value::Array(items);
    }
    Value::String(value.to_string())
}

fn map_openclaw_kind(memory_type: &str) -> MemoryKind {
    match memory_type.trim().to_ascii_lowercase().as_str() {
        "analysis" => MemoryKind::Fact,
        "task" => MemoryKind::Task,
        "decision" => MemoryKind::Decision,
        "project" => MemoryKind::ContentProject,
        "url" => MemoryKind::Url,
        _ => MemoryKind::Document,
    }
}

fn map_engram_observation_kind(observation_type: &str) -> MemoryKind {
    match observation_type.trim().to_ascii_lowercase().as_str() {
        "decision" => MemoryKind::Decision,
        "task" => MemoryKind::Task,
        "architecture" | "discovery" | "learning" | "pattern" | "config" | "bugfix" => {
            MemoryKind::Fact
        }
        _ => MemoryKind::Document,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn imports_openclaw_markdown_files() {
        let temp = tempdir().expect("test assertion");
        let file = temp.path().join("memory").join("decision.md");
        std::fs::create_dir_all(file.parent().expect("test assertion")).expect("test assertion");
        std::fs::write(
            &file,
            r#"---
title: "Auth decision"
date: "2026-03-20"
memory_type: "decision"
projects: ["xavier"]
---
Use token auth for local workflows.
"#,
        )
        .expect("test assertion");

        let memory = QmdMemory::new_with_workspace(Arc::new(RwLock::new(Vec::new())), "ws-1");
        let stats =
            import_openclaw_markdown_dir(&memory, temp.path(), BridgeImportOptions::default())
                .await
                .expect("test assertion");

        assert_eq!(stats.imported, 1);
        let docs = memory
            .search("token auth", 5)
            .await
            .expect("test assertion");
        assert_eq!(docs[0].metadata["kind"], "decision");
        assert_eq!(docs[0].metadata["provenance"]["source_app"], "openclaw");
    }

    #[tokio::test]
    async fn imports_engram_export_records() {
        let memory = QmdMemory::new_with_workspace(Arc::new(RwLock::new(Vec::new())), "ws-1");
        let export = json!({
            "sessions": [{
                "id": "session-1",
                "project": "xavier",
                "directory": "E:/scripts-python/xavier",
                "started_at": "2026-03-20T10:00:00Z",
                "summary": "Implemented typed memory"
            }],
            "observations": [{
                "id": 7,
                "session_id": "session-1",
                "type": "decision",
                "title": "Typed memory schema",
                "content": "Use canonical kinds and provenance.",
                "topic_key": "architecture/typed-memory",
                "tool_name": "codex",
                "created_at": "2026-03-20T10:05:00Z"
            }],
            "user_prompts": [{
                "id": 3,
                "session_id": "session-1",
                "content": "Implement typed memory",
                "created_at": "2026-03-20T10:01:00Z"
            }]
        });

        let stats = import_engram_export(&memory, &export, BridgeImportOptions::default())
            .await
            .expect("test assertion");

        assert_eq!(stats.imported, 3);
        let docs = memory
            .search_filtered(
                "typed memory",
                10,
                Some(&crate::memory::schema::MemoryQueryFilters {
                    session_id: Some("session-1".to_string()),
                    source_app: Some("engram".to_string()),
                    ..crate::memory::schema::MemoryQueryFilters::default()
                }),
            )
            .await
            .expect("test assertion");
        assert!(!docs.is_empty());
    }
}
