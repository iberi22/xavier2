use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Episodic,
    Semantic,
    Procedural,
    Belief,
    Org,
    Workspace,
    User,
    Agent,
    Session,
    Event,
    Fact,
    Decision,
    Repo,
    Branch,
    File,
    Symbol,
    Url,
    Task,
    Contact,
    Meeting,
    ContentProject,
    VideoAsset,
    Document,
}

impl MemoryKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "episodic" => Some(Self::Episodic),
            "semantic" => Some(Self::Semantic),
            "procedural" => Some(Self::Procedural),
            "belief" => Some(Self::Belief),
            "org" | "organization" => Some(Self::Org),
            "workspace" => Some(Self::Workspace),
            "user" => Some(Self::User),
            "agent" => Some(Self::Agent),
            "session" => Some(Self::Session),
            "event" => Some(Self::Event),
            "fact" => Some(Self::Fact),
            "decision" => Some(Self::Decision),
            "repo" | "repository" => Some(Self::Repo),
            "branch" => Some(Self::Branch),
            "file" => Some(Self::File),
            "symbol" => Some(Self::Symbol),
            "url" => Some(Self::Url),
            "task" => Some(Self::Task),
            "contact" => Some(Self::Contact),
            "meeting" => Some(Self::Meeting),
            "content_project" | "content-project" | "content project" => Some(Self::ContentProject),
            "video_asset" | "video-asset" | "video asset" => Some(Self::VideoAsset),
            "document" | "memory" | "note" => Some(Self::Document),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Belief => "belief",
            Self::Org => "org",
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::Agent => "agent",
            Self::Session => "session",
            Self::Event => "event",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Repo => "repo",
            Self::Branch => "branch",
            Self::File => "file",
            Self::Symbol => "symbol",
            Self::Url => "url",
            Self::Task => "task",
            Self::Contact => "contact",
            Self::Meeting => "meeting",
            Self::ContentProject => "content_project",
            Self::VideoAsset => "video_asset",
            Self::Document => "document",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    SourceTurn,
    SessionSummary,
    TemporalEvent,
    FactAtom,
    EntityState,
    SummaryFact,
    Observation,
    UserPrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MemoryLevel {
    #[default]
    Raw,        // Original raw memory
    Processed,  // Cleaned/standardized
    Extracted,  // Entity/relationship extracted
    Belief,     // Validated belief
}


impl MemoryLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Processed => "processed",
            Self::Extracted => "extracted",
            Self::Belief => "belief",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "processed" => Self::Processed,
            "extracted" => Self::Extracted,
            "belief" => Self::Belief,
            _ => Self::Raw,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationKind {
    pub name: String,
    pub inverse: Option<String>,
}

impl RelationKind {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inverse: None,
        }
    }
}

impl EvidenceKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "source_turn" | "source-turn" | "turn" => Some(Self::SourceTurn),
            "session_summary" | "session-summary" => Some(Self::SessionSummary),
            "temporal_event" | "temporal-event" => Some(Self::TemporalEvent),
            "fact_atom" | "fact-atom" => Some(Self::FactAtom),
            "entity_state" | "entity-state" => Some(Self::EntityState),
            "summary_fact" | "summary-fact" => Some(Self::SummaryFact),
            "observation" => Some(Self::Observation),
            "user_prompt" | "user-prompt" | "prompt" => Some(Self::UserPrompt),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceTurn => "source_turn",
            Self::SessionSummary => "session_summary",
            Self::TemporalEvent => "temporal_event",
            Self::FactAtom => "fact_atom",
            Self::EntityState => "entity_state",
            Self::SummaryFact => "summary_fact",
            Self::Observation => "observation",
            Self::UserPrompt => "user_prompt",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryNamespace {
    pub org_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub project: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub source_app: Option<String>,
    pub source_type: Option<String>,
    pub repo_url: Option<String>,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub url: Option<String>,
    pub message_id: Option<String>,
    pub created_by: Option<String>,
    pub observed_at: Option<String>,
    pub recorded_at: Option<String>,
    pub topic_key: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMemoryPayload {
    pub kind: Option<MemoryKind>,
    pub evidence_kind: Option<EvidenceKind>,
    pub namespace: Option<MemoryNamespace>,
    pub provenance: Option<MemoryProvenance>,
    pub cluster_id: Option<String>,
    pub level: Option<MemoryLevel>,
    pub relation: Option<RelationKind>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryQueryFilters {
    pub kinds: Option<Vec<MemoryKind>>,
    pub evidence_kinds: Option<Vec<EvidenceKind>>,
    pub org_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub project: Option<String>,
    pub scope: Option<String>,
    pub source_app: Option<String>,
    pub source_type: Option<String>,
    pub repo_url: Option<String>,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub url: Option<String>,
    pub message_id: Option<String>,
    pub topic_key: Option<String>,
    pub observed_after: Option<String>,
    pub observed_before: Option<String>,
    pub recorded_after: Option<String>,
    pub recorded_before: Option<String>,
    pub cluster_ids: Option<Vec<String>>,
    pub levels: Option<Vec<MemoryLevel>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMemoryMetadata {
    pub kind: MemoryKind,
    pub evidence_kind: Option<EvidenceKind>,
    pub namespace: MemoryNamespace,
    pub provenance: MemoryProvenance,
}

pub fn normalize_metadata(
    path: &str,
    metadata: Value,
    workspace_id: &str,
    typed: Option<&TypedMemoryPayload>,
) -> Result<Value> {
    let mut metadata = ensure_object(metadata);
    let resolved = resolve_metadata(path, &metadata, workspace_id, typed)?;
    metadata["kind"] = json!(resolved.kind.as_str());
    if let Some(evidence_kind) = resolved.evidence_kind {
        metadata["evidence_kind"] = json!(evidence_kind.as_str());
        if metadata.get("memory_kind").is_none() {
            metadata["memory_kind"] = json!(evidence_kind.as_str());
        }
    }
    metadata["namespace"] = serde_json::to_value(&resolved.namespace)?;
    metadata["provenance"] = serde_json::to_value(&resolved.provenance)?;
    Ok(metadata)
}

pub fn resolve_metadata(
    path: &str,
    metadata: &Value,
    workspace_id: &str,
    typed: Option<&TypedMemoryPayload>,
) -> Result<ResolvedMemoryMetadata> {
    let typed = typed.cloned().unwrap_or_default();
    let mut namespace = typed
        .namespace
        .or_else(|| {
            serde_json::from_value(metadata.get("namespace").cloned().unwrap_or(Value::Null)).ok()
        })
        .unwrap_or_default();
    if namespace.workspace_id.is_none() {
        namespace.workspace_id = Some(workspace_id.to_string());
    }
    overlay_namespace(&mut namespace, metadata);

    let mut provenance = typed
        .provenance
        .or_else(|| {
            serde_json::from_value(metadata.get("provenance").cloned().unwrap_or(Value::Null)).ok()
        })
        .unwrap_or_default();
    overlay_provenance(&mut provenance, metadata, path);
    sanitize_provenance_timestamps(&mut provenance); // auto-fix invalid timestamp slugs
    validate_timestamps(&provenance)?;

    let evidence_kind = typed
        .evidence_kind
        .or_else(|| parse_evidence_kind(metadata.get("evidence_kind")))
        .or_else(|| parse_evidence_kind(metadata.get("memory_kind")));
    let kind = typed
        .kind
        .or_else(|| parse_kind(metadata.get("kind")))
        .or_else(|| parse_kind(metadata.get("memory_type")))
        .or_else(|| parse_kind(metadata.get("type")))
        .or_else(|| infer_kind_from_evidence(evidence_kind))
        .or_else(|| infer_kind_from_path(path))
        .unwrap_or(MemoryKind::Document);

    Ok(ResolvedMemoryMetadata {
        kind,
        evidence_kind,
        namespace,
        provenance,
    })
}

pub fn matches_filters(
    path: &str,
    metadata: &Value,
    workspace_id: &str,
    filters: Option<&MemoryQueryFilters>,
) -> bool {
    let Some(filters) = filters else {
        return true;
    };
    let resolved = match resolve_metadata(path, metadata, workspace_id, None) {
        Ok(resolved) => resolved,
        Err(_) => return false,
    };

    if let Some(kinds) = &filters.kinds {
        if !kinds.contains(&resolved.kind) {
            return false;
        }
    }

    if let Some(evidence_kinds) = &filters.evidence_kinds {
        if !resolved
            .evidence_kind
            .is_some_and(|evidence_kind| evidence_kinds.contains(&evidence_kind))
        {
            return false;
        }
    }

    if !matches_string_filter(
        filters.org_id.as_deref(),
        resolved.namespace.org_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.workspace_id.as_deref(),
        resolved.namespace.workspace_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.user_id.as_deref(),
        resolved.namespace.user_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.agent_id.as_deref(),
        resolved.namespace.agent_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.session_id.as_deref(),
        resolved.namespace.session_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.project.as_deref(),
        resolved.namespace.project.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.scope.as_deref(),
        resolved.namespace.scope.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.source_app.as_deref(),
        resolved.provenance.source_app.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.source_type.as_deref(),
        resolved.provenance.source_type.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.repo_url.as_deref(),
        resolved.provenance.repo_url.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.file_path.as_deref(),
        resolved.provenance.file_path.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.symbol.as_deref(),
        resolved.provenance.symbol.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(filters.url.as_deref(), resolved.provenance.url.as_deref()) {
        return false;
    }
    if !matches_string_filter(
        filters.message_id.as_deref(),
        resolved.provenance.message_id.as_deref(),
    ) {
        return false;
    }
    if !matches_string_filter(
        filters.topic_key.as_deref(),
        resolved.provenance.topic_key.as_deref(),
    ) {
        return false;
    }

    if !matches_time_window(
        resolved.provenance.observed_at.as_deref(),
        filters.observed_after.as_deref(),
        filters.observed_before.as_deref(),
    ) {
        return false;
    }
    if !matches_time_window(
        resolved.provenance.recorded_at.as_deref(),
        filters.recorded_after.as_deref(),
        filters.recorded_before.as_deref(),
    ) {
        return false;
    }

    if let Some(cluster_ids) = &filters.cluster_ids {
        let actual_cluster = metadata.get("cluster_id").and_then(|v| v.as_str());
        if !actual_cluster.is_some_and(|cluster| cluster_ids.contains(&cluster.to_string())) {
            return false;
        }
    }

    if let Some(levels) = &filters.levels {
        let actual_level = metadata
            .get("level")
            .and_then(|v| v.as_str())
            .map(MemoryLevel::parse);
        if !actual_level.is_some_and(|level| levels.contains(&level)) {
            return false;
        }
    }

    true
}

fn ensure_object(metadata: Value) -> Value {
    match metadata {
        Value::Object(_) => metadata,
        Value::Null => json!({}),
        other => json!({ "legacy_metadata": other }),
    }
}

fn parse_kind(value: Option<&Value>) -> Option<MemoryKind> {
    value
        .and_then(|value| value.as_str())
        .and_then(MemoryKind::parse)
}

fn parse_evidence_kind(value: Option<&Value>) -> Option<EvidenceKind> {
    value
        .and_then(|value| value.as_str())
        .and_then(EvidenceKind::parse)
}

fn infer_kind_from_evidence(evidence_kind: Option<EvidenceKind>) -> Option<MemoryKind> {
    match evidence_kind {
        Some(EvidenceKind::TemporalEvent) => Some(MemoryKind::Event),
        Some(EvidenceKind::FactAtom)
        | Some(EvidenceKind::EntityState)
        | Some(EvidenceKind::SummaryFact)
        | Some(EvidenceKind::Observation) => Some(MemoryKind::Fact),
        Some(EvidenceKind::SourceTurn)
        | Some(EvidenceKind::SessionSummary)
        | Some(EvidenceKind::UserPrompt) => Some(MemoryKind::Session),
        None => None,
    }
}

fn infer_kind_from_path(path: &str) -> Option<MemoryKind> {
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("/repo") || lowered.starts_with("repo/") {
        Some(MemoryKind::Repo)
    } else if lowered.contains("/branch") || lowered.starts_with("branch/") {
        Some(MemoryKind::Branch)
    } else if lowered.contains("/symbol") || lowered.starts_with("symbol/") {
        Some(MemoryKind::Symbol)
    } else if lowered.contains("/session") || lowered.starts_with("session/") {
        Some(MemoryKind::Session)
    } else if lowered.contains("/task") || lowered.starts_with("task/") {
        Some(MemoryKind::Task)
    } else if lowered.contains("/meeting") || lowered.starts_with("meeting/") {
        Some(MemoryKind::Meeting)
    } else if lowered.contains("/url") || lowered.starts_with("url/") {
        Some(MemoryKind::Url)
    } else if lowered.contains('.') || lowered.contains("/file") || lowered.starts_with("file/") {
        Some(MemoryKind::File)
    } else {
        None
    }
}

fn overlay_namespace(namespace: &mut MemoryNamespace, metadata: &Value) {
    namespace.org_id = namespace
        .org_id
        .take()
        .or_else(|| string_value(metadata, "org_id"));
    namespace.workspace_id = namespace
        .workspace_id
        .take()
        .or_else(|| string_value(metadata, "workspace_id"));
    namespace.user_id = namespace
        .user_id
        .take()
        .or_else(|| string_value(metadata, "user_id"));
    namespace.agent_id = namespace
        .agent_id
        .take()
        .or_else(|| string_value(metadata, "agent_id"));
    namespace.session_id = namespace
        .session_id
        .take()
        .or_else(|| string_value(metadata, "session_id"));
    namespace.project = namespace
        .project
        .take()
        .or_else(|| string_value(metadata, "project"))
        .or_else(|| first_string(metadata, "projects"));
    namespace.scope = namespace
        .scope
        .take()
        .or_else(|| string_value(metadata, "scope"))
        .or_else(|| string_value(metadata, "gestalt_context"));
}

fn overlay_provenance(provenance: &mut MemoryProvenance, metadata: &Value, path: &str) {
    provenance.source_app = provenance
        .source_app
        .take()
        .or_else(|| string_value(metadata, "source_app"));
    provenance.source_type = provenance
        .source_type
        .take()
        .or_else(|| string_value(metadata, "source_type"))
        .or_else(|| string_value(metadata, "type"));
    provenance.repo_url = provenance
        .repo_url
        .take()
        .or_else(|| string_value(metadata, "repo_url"));
    provenance.file_path = provenance
        .file_path
        .take()
        .or_else(|| string_value(metadata, "file_path"))
        .or_else(|| string_value(metadata, "source_path"))
        .or_else(|| Some(path.to_string()));
    provenance.symbol = provenance
        .symbol
        .take()
        .or_else(|| string_value(metadata, "symbol"));
    provenance.url = provenance
        .url
        .take()
        .or_else(|| string_value(metadata, "url"));
    provenance.message_id = provenance
        .message_id
        .take()
        .or_else(|| string_value(metadata, "message_id"));
    provenance.created_by = provenance
        .created_by
        .take()
        .or_else(|| string_value(metadata, "created_by"));
    provenance.observed_at = provenance
        .observed_at
        .take()
        .or_else(|| string_value(metadata, "observed_at"))
        .or_else(|| string_value(metadata, "date"));
    provenance.recorded_at = provenance
        .recorded_at
        .take()
        .or_else(|| string_value(metadata, "recorded_at"));
    provenance.topic_key = provenance
        .topic_key
        .take()
        .or_else(|| string_value(metadata, "topic_key"));
    provenance.tool_name = provenance
        .tool_name
        .take()
        .or_else(|| string_value(metadata, "tool_name"));
}

/// Attempt to parse a timestamp string into a fixed ISO8601 string.
/// Returns None if the value is empty or cannot be parsed.
fn sanitize_timestamp(value: Option<&str>) -> Option<String> {
    let value = value?;
    if value.is_empty() {
        return None;
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc).to_rfc3339());
    }
    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let naive = parsed.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).to_rfc3339());
    }
    // Could not parse — auto-generate current timestamp as fallback
    Some(Utc::now().to_rfc3339())
}

fn validate_timestamps(provenance: &MemoryProvenance) -> Result<()> {
    for value in [&provenance.observed_at, &provenance.recorded_at]
        .into_iter()
        .flatten()
    {
        parse_time(value)?;
    }
    Ok(())
}

/// Sanitize timestamp fields in provenance, replacing unparseable slugs
/// with a server-generated ISO8601 timestamp.
pub fn sanitize_provenance_timestamps(provenance: &mut MemoryProvenance) {
    if let Some(fixed) = sanitize_timestamp(provenance.observed_at.as_deref()) {
        provenance.observed_at = Some(fixed);
    }
    if let Some(fixed) = sanitize_timestamp(provenance.recorded_at.as_deref()) {
        provenance.recorded_at = Some(fixed);
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }
    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let naive = parsed
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow!("invalid date value: {value}"))?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    Err(anyhow!("invalid timestamp value: {value}"))
}

fn matches_time_window(value: Option<&str>, after: Option<&str>, before: Option<&str>) -> bool {
    if after.is_none() && before.is_none() {
        return true;
    }
    let Some(value) = value else {
        return false;
    };
    let Ok(moment) = parse_time(value) else {
        return false;
    };

    if let Some(after) = after {
        let Ok(after) = parse_time(after) else {
            return false;
        };
        if moment < after {
            return false;
        }
    }

    if let Some(before) = before {
        let Ok(before) = parse_time(before) else {
            return false;
        };
        if moment > before {
            return false;
        }
    }

    true
}

fn matches_string_filter(expected: Option<&str>, actual: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    actual.is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
}

fn string_value(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
}

fn first_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_metadata_into_canonical_fields() {
        let metadata = normalize_metadata(
            "repo/xavier/src/lib.rs",
            json!({
                "memory_kind": "fact_atom",
                "session_id": "s-1",
                "user_id": "u-1",
                "source_app": "engram",
                "source_type": "observation",
                "file_path": "src/lib.rs"
            }),
            "workspace-a",
            None,
        )
        .expect("test assertion");

        assert_eq!(metadata["kind"], "fact");
        assert_eq!(metadata["evidence_kind"], "fact_atom");
        assert_eq!(metadata["namespace"]["workspace_id"], "workspace-a");
        assert_eq!(metadata["namespace"]["session_id"], "s-1");
        assert_eq!(metadata["provenance"]["source_app"], "engram");
    }

    #[test]
    fn filters_match_namespace_and_provenance() {
        let metadata = normalize_metadata(
            "docs/api",
            json!({
                "kind": "file",
                "namespace": {
                    "workspace_id": "ws-1",
                    "session_id": "session-42"
                },
                "provenance": {
                    "source_app": "openclaw",
                    "file_path": "memory/api.md"
                }
            }),
            "ws-1",
            None,
        )
        .expect("test assertion");

        assert!(matches_filters(
            "docs/api",
            &metadata,
            "ws-1",
            Some(&MemoryQueryFilters {
                kinds: Some(vec![MemoryKind::File]),
                session_id: Some("session-42".to_string()),
                source_app: Some("openclaw".to_string()),
                file_path: Some("memory/api.md".to_string()),
                ..MemoryQueryFilters::default()
            })
        ));
    }

    #[test]
    fn sanitizes_invalid_timestamp_values() {
        let metadata = normalize_metadata(
            "docs/api",
            json!({
                "provenance": {
                    "recorded_at": "not-a-date"
                }
            }),
            "ws-1",
            None,
        )
        .expect("test assertion");

        let recorded_at = metadata["provenance"]["recorded_at"]
            .as_str()
            .expect("test assertion");
        assert!(DateTime::parse_from_rfc3339(recorded_at).is_ok());
        assert_ne!(recorded_at, "not-a-date");
    }
}
