use crate::{
    memory::schema::{
        EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryQueryFilters,
        TypedMemoryPayload,
    },
    utils::crypto::hex_encode,
    workspace::WorkspaceContext,
    AppState,
};
use axum::{
    body::Bytes,
    extract::State,
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::info;
use ulid::Ulid;

// ============================================================================
// MCP JSON-RPC Types
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MCPRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ============================================================================
// MCP Payload Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPToolResult {
    pub content: Vec<MCPTextContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPTextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// ============================================================================
// Xavier2 MCP Tools
// ============================================================================

pub fn get_xavier2_tools() -> Vec<MCPTool> {
    vec![
        MCPTool {
            name: "create_memory".to_string(),
            description: "Create a new memory document in Xavier2".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path/identifier for the memory"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content of the memory"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Optional metadata"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Canonical memory kind"
                    },
                    "evidence_kind": {
                        "type": "string",
                        "description": "Optional retrieval evidence kind"
                    },
                    "namespace": {
                        "type": "object",
                        "description": "Namespace fields for org/workspace/user/agent/session"
                    },
                    "provenance": {
                        "type": "object",
                        "description": "Source and provenance fields"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        MCPTool {
            name: "search_memory".to_string(),
            description: "Search memory documents in Xavier2".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum results",
                        "default": 10
                    },
                    "filters": {
                        "type": "object",
                        "description": "Optional namespace/provenance filters"
                    }
                },
                "required": ["query"]
            }),
        },
        MCPTool {
            name: "get_memory".to_string(),
            description: "Get a specific memory by ID".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Memory ID"
                    }
                },
                "required": ["id"]
            }),
        },
        MCPTool {
            name: "list_projects".to_string(),
            description: "List all projects in Xavier2".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        MCPTool {
            name: "get_project_context".to_string(),
            description: "Get full context for a project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project identifier"
                    }
                },
                "required": ["project_id"]
            }),
        },
        MCPTool {
            name: "sync_gitcore".to_string(),
            description: "Sync documentation from GitCore project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {
                        "type": "string",
                        "description": "Path to GitCore project"
                    }
                },
                "required": ["project_path"]
            }),
        },
    ]
}

pub fn get_xavier2_resources() -> Vec<MCPResource> {
    vec![
        MCPResource {
            uri: "xavier2://memory".to_string(),
            name: "Memory Store".to_string(),
            mime_type: "application/json".to_string(),
        },
        MCPResource {
            uri: "xavier2://projects".to_string(),
            name: "Projects List".to_string(),
            mime_type: "application/json".to_string(),
        },
        MCPResource {
            uri: "xavier2://health".to_string(),
            name: "System Health".to_string(),
            mime_type: "application/json".to_string(),
        },
    ]
}

// ============================================================================
// MCP Transport Handlers
// ============================================================================

const MCP_SESSION_HEADER: &str = "mcp-session-id";

pub async fn mcp_post_handler(
    State(state): State<AppState>,
    Extension(workspace): Extension<WorkspaceContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON payload: {error}"),
            )
                .into_response();
        }
    };

    let session_header = match resolve_mcp_session_header(&headers, &payload) {
        Ok(value) => value,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };

    match dispatch_mcp_value(state, workspace, payload).await {
        Ok(Some(response)) => with_session_header(
            (StatusCode::OK, Json(response)).into_response(),
            session_header,
        ),
        Ok(None) => with_session_header(StatusCode::ACCEPTED.into_response(), session_header),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn mcp_get_handler() -> impl IntoResponse {
    StatusCode::METHOD_NOT_ALLOWED
}

pub async fn mcp_delete_handler() -> impl IntoResponse {
    StatusCode::METHOD_NOT_ALLOWED
}

fn resolve_mcp_session_header(
    headers: &HeaderMap,
    payload: &Value,
) -> Result<Option<HeaderValue>, String> {
    if let Some(value) = headers.get(MCP_SESSION_HEADER) {
        if value.as_bytes().is_empty() {
            return Err("Mcp-Session-Id header must not be empty".to_string());
        }
        return Ok(Some(value.clone()));
    }

    if payload_method(payload).is_some_and(|method| method == "initialize") {
        let session_id = format!("xavier2-{}", Ulid::new());
        let value = HeaderValue::from_str(&session_id)
            .map_err(|_| "Failed to generate MCP session header".to_string())?;
        return Ok(Some(value));
    }

    Ok(None)
}

fn payload_method(payload: &Value) -> Option<&str> {
    match payload {
        Value::Object(map) => map.get("method").and_then(|value| value.as_str()),
        Value::Array(items) => items.iter().find_map(payload_method),
        _ => None,
    }
}

fn with_session_header(mut response: Response, session_header: Option<HeaderValue>) -> Response {
    if let Some(value) = session_header {
        response
            .headers_mut()
            .insert(HeaderName::from_static(MCP_SESSION_HEADER), value);
    }
    response
}

pub async fn dispatch_mcp_value(
    state: AppState,
    workspace: WorkspaceContext,
    payload: Value,
) -> Result<Option<Value>, String> {
    match payload {
        Value::Array(messages) => {
            if messages.is_empty() {
                return Err("Invalid JSON-RPC batch: empty batch".to_string());
            }

            let mut responses = Vec::new();
            for message in messages {
                if let Some(response) =
                    dispatch_mcp_message(state.clone(), workspace.clone(), message).await?
                {
                    responses.push(serde_json::to_value(response).map_err(|e| e.to_string())?);
                }
            }

            if responses.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Value::Array(responses)))
            }
        }
        message => dispatch_mcp_message(state, workspace, message)
            .await?
            .map(|response| serde_json::to_value(response).map_err(|e| e.to_string()))
            .transpose(),
    }
}

async fn dispatch_mcp_message(
    state: AppState,
    workspace: WorkspaceContext,
    message: Value,
) -> Result<Option<MCPResponse>, String> {
    let object = message
        .as_object()
        .ok_or_else(|| "Invalid JSON-RPC message: expected object or batch".to_string())?;

    match classify_message(object)? {
        IncomingKind::Request => {
            let request: MCPRequest =
                serde_json::from_value(Value::Object(object.clone())).map_err(|e| e.to_string())?;
            handle_mcp_request(state, workspace, request).await
        }
        IncomingKind::Response => Ok(None),
    }
}

enum IncomingKind {
    Request,
    Response,
}

fn classify_message(object: &serde_json::Map<String, Value>) -> Result<IncomingKind, String> {
    match object.get("jsonrpc").and_then(|value| value.as_str()) {
        Some("2.0") => {}
        _ => return Err("Invalid JSON-RPC message: jsonrpc must be \"2.0\"".to_string()),
    }

    if object.contains_key("method") {
        return Ok(IncomingKind::Request);
    }

    if object.contains_key("result") || object.contains_key("error") {
        return Ok(IncomingKind::Response);
    }

    Err("Invalid JSON-RPC message: missing method/result/error".to_string())
}

async fn handle_mcp_request(
    state: AppState,
    workspace: WorkspaceContext,
    request: MCPRequest,
) -> Result<Option<MCPResponse>, String> {
    let request_id = request.id.clone();
    let is_notification = request_id.is_none();

    if request.jsonrpc != "2.0" {
        return Ok(error_response(
            request_id,
            -32600,
            "Invalid Request".to_string(),
        ));
    }

    info!(method = %request.method, notification = is_notification, "mcp_request");

    let response = match request.method.as_str() {
        "initialize" => Some(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.unwrap_or(Value::Null),
            result: Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "xavier2-memory",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        }),
        "notifications/initialized" => None,
        "resources/list" => Some(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.unwrap_or(Value::Null),
            result: Some(json!({
                "resources": get_xavier2_resources()
            })),
            error: None,
        }),
        "tools/list" => Some(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.unwrap_or(Value::Null),
            result: Some(json!({
                "tools": get_xavier2_tools()
            })),
            error: None,
        }),
        "tools/call" => {
            let params = request.params.unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            Some(
                match handle_tool_call(state, workspace, name, arguments).await {
                    Ok(result) => MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.unwrap_or(Value::Null),
                        result: Some(result),
                        error: None,
                    },
                    Err(error) => MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.unwrap_or(Value::Null),
                        result: None,
                        error: Some(MCPError {
                            code: -32000,
                            message: error.to_string(),
                            data: None,
                        }),
                    },
                },
            )
        }
        _ if is_notification => None,
        _ => error_response(
            request.id,
            -32601,
            format!("Method not found: {}", request.method),
        ),
    };

    if is_notification {
        Ok(None)
    } else {
        Ok(response)
    }
}

fn error_response(id: Option<Value>, code: i32, message: String) -> Option<MCPResponse> {
    Some(MCPResponse {
        jsonrpc: "2.0".to_string(),
        id: id.unwrap_or(Value::Null),
        result: None,
        error: Some(MCPError {
            code,
            message,
            data: None,
        }),
    })
}

pub async fn handle_tool_call(
    _state: AppState,
    workspace: WorkspaceContext,
    name: &str,
    arguments: Value,
) -> anyhow::Result<Value> {
    match name {
        "search_memory" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let filters = arguments
                .get("filters")
                .cloned()
                .map(serde_json::from_value::<MemoryQueryFilters>)
                .transpose()?;

            let results = workspace
                .workspace
                .memory
                .search_filtered(query, limit, filters.as_ref())
                .await?;
            let content = results
                .into_iter()
                .map(|doc| MCPTextContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "Path: {}\nContent: {}\nMetadata: {:?}",
                        doc.path, doc.content, doc.metadata
                    ),
                })
                .collect();

            Ok(serde_json::to_value(MCPToolResult {
                content,
                is_error: Some(false),
            })?)
        }
        "get_memory" => {
            let id = arguments
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
            let record = workspace
                .workspace
                .get_memory_record(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Memory not found: {id}"))?;

            Ok(serde_json::to_value(MCPToolResult {
                content: vec![MCPTextContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "Id: {}\nPath: {}\nRevision: {}\nPrimary: {}\nContent: {}\nMetadata: {}",
                        record.id,
                        record.path,
                        record.revision,
                        record.primary,
                        record.content,
                        serde_json::to_string_pretty(&record.metadata)?
                    ),
                }],
                is_error: Some(false),
            })?)
        }
        "create_memory" => {
            let path = arguments
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
            let content = arguments
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
            let metadata = arguments
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let kind = arguments
                .get("kind")
                .cloned()
                .map(serde_json::from_value::<MemoryKind>)
                .transpose()?;
            let evidence_kind = arguments
                .get("evidence_kind")
                .cloned()
                .map(serde_json::from_value::<EvidenceKind>)
                .transpose()?;
            let namespace = arguments
                .get("namespace")
                .cloned()
                .map(serde_json::from_value::<MemoryNamespace>)
                .transpose()?;
            let provenance = arguments
                .get("provenance")
                .cloned()
                .map(serde_json::from_value::<MemoryProvenance>)
                .transpose()?;

            workspace
                .workspace
                .ingest_typed(
                    path.to_string(),
                    content.to_string(),
                    metadata,
                    Some(TypedMemoryPayload {
                        kind,
                        evidence_kind,
                        namespace,
                        provenance,
                    }),
                    None,
                    false,
                )
                .await?;

            Ok(serde_json::to_value(MCPToolResult {
                content: vec![MCPTextContent {
                    content_type: "text".to_string(),
                    text: format!("Memory created successfully at path: {}", path),
                }],
                is_error: Some(false),
            })?)
        }
        "list_projects" => {
            let records = workspace.workspace.list_memory_records().await?;
            let mut projects = std::collections::BTreeMap::<String, usize>::new();

            for record in records {
                if let Ok(resolved) = crate::memory::schema::resolve_metadata(
                    &record.path,
                    &record.metadata,
                    &workspace.workspace_id,
                    None,
                ) {
                    if let Some(project) = resolved.namespace.project {
                        *projects.entry(project).or_insert(0) += 1;
                    }
                }
            }

            let text = if projects.is_empty() {
                "No projects found.".to_string()
            } else {
                projects
                    .into_iter()
                    .map(|(project, count)| format!("{project}: {count} memories"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            Ok(serde_json::to_value(MCPToolResult {
                content: vec![MCPTextContent {
                    content_type: "text".to_string(),
                    text,
                }],
                is_error: Some(false),
            })?)
        }
        "get_project_context" => {
            let project_id = arguments
                .get("project_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing project_id"))?;
            let records = workspace.workspace.list_memory_records().await?;
            let matching = records
                .into_iter()
                .filter(|record| {
                    crate::memory::schema::resolve_metadata(
                        &record.path,
                        &record.metadata,
                        &workspace.workspace_id,
                        None,
                    )
                    .ok()
                    .and_then(|resolved| resolved.namespace.project)
                    .as_deref()
                        == Some(project_id)
                })
                .take(20)
                .map(|record| {
                    format!(
                        "Id: {}\nPath: {}\nRevision: {}\nContent: {}",
                        record.id, record.path, record.revision, record.content
                    )
                })
                .collect::<Vec<_>>();

            Ok(serde_json::to_value(MCPToolResult {
                content: vec![MCPTextContent {
                    content_type: "text".to_string(),
                    text: if matching.is_empty() {
                        format!("No context found for project {project_id}.")
                    } else {
                        matching.join("\n\n---\n\n")
                    },
                }],
                is_error: Some(false),
            })?)
        }
        "sync_gitcore" => {
            let project_path = arguments
                .get("project_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing project_path"))?;
            let root = std::path::PathBuf::from(project_path);
            let mut created = 0usize;
            let mut updated = 0usize;
            let mut unchanged = 0usize;
            let mut skipped = 0usize;

            for relative in ["AGENTS.md", ".gitcore/ARCHITECTURE.md", "README.md"] {
                let candidate = root.join(relative);
                if !tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                    skipped += 1;
                    continue;
                }

                let content = tokio::fs::read_to_string(&candidate).await?;
                let project = root
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("gitcore");
                let path = format!("gitcore/{project}/{}", relative.replace('\\', "/"));
                let content_hash = hex_encode(&Sha256::digest(content.as_bytes()));
                let metadata = json!({
                    "synced_from": candidate.display().to_string(),
                    "content_hash": content_hash,
                });
                let typed = Some(TypedMemoryPayload {
                    kind: Some(MemoryKind::Document),
                    evidence_kind: Some(EvidenceKind::Observation),
                    namespace: Some(MemoryNamespace {
                        project: Some(project.to_string()),
                        ..MemoryNamespace::default()
                    }),
                    provenance: Some(MemoryProvenance {
                        source_app: Some("gitcore".to_string()),
                        source_type: Some("repository_doc".to_string()),
                        file_path: Some(relative.replace('\\', "/")),
                        ..MemoryProvenance::default()
                    }),
                });

                if let Some(existing) = workspace.workspace.get_memory_record(&path).await? {
                    let existing_hash = existing
                        .metadata
                        .get("content_hash")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    if existing_hash == content_hash && existing.content == content {
                        unchanged += 1;
                        continue;
                    }

                    workspace
                        .workspace
                        .update_primary_memory(&existing.id, path, content, metadata, typed)
                        .await?;
                    updated += 1;
                    continue;
                }

                workspace
                    .workspace
                    .ingest_typed(path, content, metadata, typed, None, false)
                    .await?;
                created += 1;
            }

            Ok(serde_json::to_value(MCPToolResult {
                content: vec![MCPTextContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "Synced GitCore documents from {project_path}\ncreated={created}\nupdated={updated}\nunchanged={unchanged}\nskipped={skipped}"
                    ),
                }],
                is_error: Some(false),
            })?)
        }
        _ => Err(anyhow::anyhow!("Tool not implemented: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request},
        routing::post,
        Router,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    use crate::{
        agents::RuntimeConfig,
        coordination::SimpleAgentRegistry,
        memory::file_indexer::{FileIndexer, FileIndexerConfig},
        ports::inbound::NoopTimeMetricsPort,
        workspace::{WorkspaceConfig, WorkspaceRegistry, WorkspaceState},
        AppState,
    };

    fn unique_test_path(prefix: &str, suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}-{suffix}"))
    }

    async fn test_state() -> (AppState, WorkspaceContext) {
        let db_path = unique_test_path("xavier2-code-mcp", "code_graph.db");
        let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&db_path).unwrap());
        let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
        let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));
        let workspace_registry = Arc::new(WorkspaceRegistry::new());
        let workspace = WorkspaceState::new(
            WorkspaceConfig {
                id: "test".to_string(),
                token: "test-token".to_string(),
                plan: crate::workspace::PlanTier::Personal,
                memory_backend: crate::memory::surreal_store::MemoryBackend::File,
                storage_limit_bytes: Some(10 * 1024 * 1024),
                request_limit: Some(10_000),
                request_unit_limit: Some(20_000),
                embedding_provider_mode: crate::workspace::EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: crate::workspace::SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            unique_test_path("xavier2-mcp-store", "threads"),
        )
        .await
        .unwrap();
        workspace_registry.insert(workspace).await.unwrap();
        let workspace = workspace_registry.authenticate("test-token").await.unwrap();

        (
            AppState {
                workspace_id: "test".to_string(),
                workspace_registry,
                indexer: FileIndexer::new(FileIndexerConfig::default(), Some(code_indexer.clone())),
                code_indexer,
                code_query,
                code_db,
                pattern_adapter: Arc::new(
                    crate::adapters::outbound::vec::pattern_adapter::PatternAdapter::new(),
                ),
                security_service: Arc::new(crate::app::security_service::SecurityService::new()),
                time_metrics: Arc::new(NoopTimeMetricsPort),
                agent_registry: SimpleAgentRegistry::new(),
            },
            workspace,
        )
    }

    fn test_router(state: AppState, workspace: WorkspaceContext) -> Router {
        Router::new()
            .route(
                "/mcp",
                post(mcp_post_handler)
                    .get(mcp_get_handler)
                    .delete(mcp_delete_handler),
            )
            .layer(Extension(workspace))
            .with_state(state)
    }

    async fn post_json(app: Router, body: Value) -> Response {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn post_json_with_headers(
        app: Router,
        body: Value,
        headers: &[(&str, &str)],
    ) -> Response {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        app.oneshot(
            request
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn initialize_returns_current_protocol_version() {
        let (state, workspace) = test_state().await;
        let response = post_json(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0" }
                }
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let session_id = response
            .headers()
            .get(MCP_SESSION_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap();
        assert!(session_id.starts_with("xavier2-"));
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(payload["result"]["serverInfo"]["name"], "xavier2-memory");
    }

    #[tokio::test]
    async fn initialized_notification_returns_accepted_with_empty_body() {
        let (state, workspace) = test_state().await;
        let response = post_json_with_headers(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }),
            &[(MCP_SESSION_HEADER, "xavier2-session-test")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response
                .headers()
                .get(MCP_SESSION_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("xavier2-session-test")
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn tool_calls_echo_existing_session_id_header() {
        let (state, workspace) = test_state().await;
        let response = post_json_with_headers(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "tools/list"
            }),
            &[(MCP_SESSION_HEADER, "xavier2-session-existing")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(MCP_SESSION_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("xavier2-session-existing")
        );
    }

    #[tokio::test]
    async fn response_only_payload_returns_accepted() {
        let (state, workspace) = test_state().await;
        let response = post_json(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "result": { "ok": true }
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn tools_list_returns_all_xavier2_tools() {
        let (state, workspace) = test_state().await;
        let response = post_json(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["result"]["tools"].as_array().unwrap().len(), 6);
    }

    #[tokio::test]
    async fn tools_call_search_memory_returns_tool_content() {
        let (state, workspace) = test_state().await;
        workspace
            .workspace
            .memory
            .add_document(
                "notes/demo".to_string(),
                "MCP transport verification document".to_string(),
                json!({}),
            )
            .await
            .unwrap();

        let response = post_json(
            test_router(state, workspace),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "search_memory",
                    "arguments": {
                        "query": "transport verification",
                        "limit": 5
                    }
                }
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let content = payload["result"]["content"].as_array().unwrap();
        assert!(!content.is_empty());
        assert!(content[0]["text"]
            .as_str()
            .unwrap()
            .contains("MCP transport verification"));
    }

    #[tokio::test]
    async fn tools_call_create_and_search_memory_support_typed_filters() {
        let (state, workspace) = test_state().await;
        let workspace_for_assertions = workspace.clone();
        let app = test_router(state, workspace);

        let create_response = post_json(
            app.clone(),
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": {
                    "name": "create_memory",
                    "arguments": {
                        "path": "bridge/openclaw/task",
                        "content": "OpenClaw imported the publishing backlog into Xavier2.",
                        "kind": "task",
                        "evidence_kind": "fact_atom",
                        "namespace": {
                            "project": "content-ops",
                            "agent_id": "openclaw-content",
                            "session_id": "handoff-7"
                        },
                        "provenance": {
                            "source_app": "openclaw",
                            "source_type": "bridge_import",
                            "topic_key": "content/youtube-backlog"
                        }
                    }
                }
            }),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::OK);

        let response = post_json(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/call",
                "params": {
                    "name": "search_memory",
                    "arguments": {
                        "query": "publishing backlog",
                        "limit": 5,
                        "filters": {
                            "project": "content-ops",
                            "agent_id": "openclaw-content",
                            "source_app": "openclaw",
                            "topic_key": "content/youtube-backlog"
                        }
                    }
                }
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let content = payload["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("Path: bridge/openclaw/task"));
        assert!(text.contains("openclaw-content"));
        assert!(text.contains("content/youtube-backlog"));

        let entity = workspace_for_assertions
            .workspace
            .entity_graph
            .entity("OpenClaw")
            .await;
        assert!(
            entity.is_some(),
            "MCP create_memory should index entity graph"
        );

        let semantic_stats = workspace_for_assertions
            .workspace
            .semantic_memory
            .stats()
            .await;
        assert!(
            semantic_stats.total_entities > 0,
            "MCP create_memory should index semantic memory"
        );
    }

    #[tokio::test]
    async fn tools_call_get_memory_and_project_context_return_durable_context() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace.clone());
        let memory_id = workspace
            .workspace
            .memory
            .add_document_typed(
                "projects/xavier2/spec".to_string(),
                "Xavier2 durable memory context".to_string(),
                json!({}),
                Some(TypedMemoryPayload {
                    kind: Some(MemoryKind::Semantic),
                    evidence_kind: Some(EvidenceKind::Observation),
                    namespace: Some(MemoryNamespace {
                        project: Some("xavier2".to_string()),
                        ..MemoryNamespace::default()
                    }),
                    provenance: None,
                }),
            )
            .await
            .unwrap();

        let get_response = post_json(
            app.clone(),
            json!({
                "jsonrpc": "2.0",
                "id": 13,
                "method": "tools/call",
                "params": {
                    "name": "get_memory",
                    "arguments": { "id": memory_id }
                }
            }),
        )
        .await;
        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert!(payload["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Revision: 1"));

        let list_projects = post_json(
            app.clone(),
            json!({
                "jsonrpc": "2.0",
                "id": 14,
                "method": "tools/call",
                "params": {
                    "name": "list_projects",
                    "arguments": {}
                }
            }),
        )
        .await;
        assert_eq!(list_projects.status(), StatusCode::OK);

        let project_context = post_json(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 15,
                "method": "tools/call",
                "params": {
                    "name": "get_project_context",
                    "arguments": { "project_id": "xavier2" }
                }
            }),
        )
        .await;
        assert_eq!(project_context.status(), StatusCode::OK);
        let body = to_bytes(project_context.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert!(payload["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Xavier2 durable memory context"));
    }

    #[tokio::test]
    async fn sync_gitcore_is_idempotent_and_revisioned() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace.clone());
        let root = unique_test_path("xavier2-gitcore", "repo");
        tokio::fs::create_dir_all(root.join(".gitcore"))
            .await
            .unwrap();
        tokio::fs::write(root.join("AGENTS.md"), "agent rules v1")
            .await
            .unwrap();
        tokio::fs::write(root.join(".gitcore/ARCHITECTURE.md"), "architecture v1")
            .await
            .unwrap();
        tokio::fs::write(root.join("README.md"), "readme v1")
            .await
            .unwrap();

        let first = post_json(
            app.clone(),
            json!({
                "jsonrpc": "2.0",
                "id": 16,
                "method": "tools/call",
                "params": {
                    "name": "sync_gitcore",
                    "arguments": {
                        "project_path": root.display().to_string()
                    }
                }
            }),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);
        let body = to_bytes(first.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let text = payload["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("created=3"));
        assert!(text.contains("updated=0"));
        assert!(text.contains("unchanged=0"));

        let second = post_json(
            app.clone(),
            json!({
                "jsonrpc": "2.0",
                "id": 17,
                "method": "tools/call",
                "params": {
                    "name": "sync_gitcore",
                    "arguments": {
                        "project_path": root.display().to_string()
                    }
                }
            }),
        )
        .await;
        assert_eq!(second.status(), StatusCode::OK);
        let body = to_bytes(second.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let text = payload["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("created=0"));
        assert!(text.contains("updated=0"));
        assert!(text.contains("unchanged=3"));

        tokio::fs::write(root.join("README.md"), "readme v2")
            .await
            .unwrap();
        let third = post_json(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 18,
                "method": "tools/call",
                "params": {
                    "name": "sync_gitcore",
                    "arguments": {
                        "project_path": root.display().to_string()
                    }
                }
            }),
        )
        .await;
        assert_eq!(third.status(), StatusCode::OK);
        let body = to_bytes(third.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let text = payload["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("updated=1"));

        let project = root.file_name().unwrap().to_str().unwrap();
        let record = workspace
            .workspace
            .get_memory_record(&format!("gitcore/{project}/README.md"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.revision, 2);
        assert_eq!(record.content, "readme v2");
    }

    #[tokio::test]
    async fn malformed_json_returns_bad_request() {
        let (state, workspace) = test_state().await;
        let response = test_router(state, workspace)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from("{not json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_and_delete_are_method_not_allowed() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let delete_response = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/mcp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
