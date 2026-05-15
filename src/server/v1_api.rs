//! V1 RESTful Standard Memory API handlers.

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    memory::{
        qmd_memory::query_with_embedding_filtered,
        schema::{
            EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryQueryFilters,
            TypedMemoryPayload,
        },
    },
    workspace::WorkspaceContext,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct V1Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct V1AddMemoryRequest {
    pub messages: Option<Vec<V1Message>>,
    pub text: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub user_id: Option<String>,
    pub kind: Option<MemoryKind>,
    pub evidence_kind: Option<EvidenceKind>,
    pub namespace: Option<MemoryNamespace>,
    pub provenance: Option<MemoryProvenance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct V1MemoryResponse {
    pub id: String,
    pub memory: String,
    pub user_id: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct V1PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct V1PaginationMetadata {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct V1MemoryListResponse {
    pub memories: Vec<V1MemoryResponse>,
    pub pagination: V1PaginationMetadata,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct V1SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub filters: Option<MemoryQueryFilters>,
}

#[derive(Debug, Serialize, Clone)]
pub struct V1MemorySearchResponse {
    pub status: String,
    pub results: Vec<V1MemoryResponse>,
}

fn is_primary_memory(metadata: &serde_json::Value) -> bool {
    metadata.get("source_path").is_none()
}

pub async fn v1_memories_add(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<V1AddMemoryRequest>,
) -> impl IntoResponse {
    info!(
        user_id = payload.user_id.as_deref().unwrap_or("default"),
        "v1_memories_add"
    );

    let content = if let Some(t) = payload.text {
        t
    } else if let Some(m) = payload.messages {
        m.into_iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };
    let content_for_graph = content.clone();

    let path = payload
        .user_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let mut meta = payload.metadata.unwrap_or(serde_json::json!({}));
    let mut namespace = payload.namespace;
    if let Some(uid) = payload.user_id {
        meta["user_id"] = serde_json::json!(uid);
        if namespace
            .as_ref()
            .and_then(|value| value.user_id.as_ref())
            .is_none()
        {
            let mut value = namespace.unwrap_or_default();
            value.user_id = meta
                .get("user_id")
                .and_then(|id| id.as_str())
                .map(|id| id.to_string());
            namespace = Some(value);
        }
    }
    let meta_for_graph = meta.clone();

    if let Err(error) = workspace
        .workspace
        .ensure_within_storage_limit(&path, &content, &meta)
        .await
    {
        return Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
        }));
    }

    match workspace
        .workspace
        .memory
        .add_document_typed(
            path,
            content,
            meta,
            Some(TypedMemoryPayload {
                kind: payload.kind,
                evidence_kind: payload.evidence_kind,
                namespace,
                provenance: payload.provenance,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(id) => {
            if let Err(error) = workspace
                .workspace
                .index_memory_entities(&id, &content_for_graph, &meta_for_graph)
                .await
            {
                tracing::warn!(%error, memory_id = %id, "failed to index entity graph from v1 add");
            }
            Json(serde_json::json!({
                "status": "ok",
                "message": "Memory added successfully",
                "id": id,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        })),
    }
}

pub async fn v1_memories_search(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<V1SearchRequest>,
) -> impl IntoResponse {
    let limit = payload.limit.unwrap_or(10);
    let results = query_with_embedding_filtered(
        &workspace.workspace.memory,
        &payload.query,
        limit,
        payload.filters.as_ref(),
    )
    .await
    .unwrap_or_default()
    .into_iter()
    .filter(|doc| is_primary_memory(&doc.metadata))
    .map(|doc| V1MemoryResponse {
        id: doc.id.unwrap_or_default(),
        memory: doc.content,
        user_id: Some(doc.path),
        metadata: doc.metadata,
    })
    .collect();

    Json(V1MemorySearchResponse {
        status: "ok".to_string(),
        results,
    })
}

pub async fn v1_memories_list(
    Extension(workspace): Extension<WorkspaceContext>,
    Query(params): Query<V1PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let all_docs: Vec<_> = workspace
        .workspace
        .memory
        .all_documents()
        .await
        .into_iter()
        .filter(|doc| is_primary_memory(&doc.metadata))
        .collect();
    let total = all_docs.len();

    let memories = all_docs
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|doc| V1MemoryResponse {
            id: doc.id.unwrap_or_default(),
            memory: doc.content,
            user_id: Some(doc.path),
            metadata: doc.metadata,
        })
        .collect();

    Json(V1MemoryListResponse {
        memories,
        pagination: V1PaginationMetadata {
            total,
            limit,
            offset,
        },
    })
}

pub async fn v1_memories_get(
    Extension(workspace): Extension<WorkspaceContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match workspace.workspace.memory.get(&id).await {
        Ok(Some(doc)) if is_primary_memory(&doc.metadata) => Json(serde_json::json!({
            "status": "ok",
            "memory": V1MemoryResponse {
                id: doc.id.unwrap_or_default(),
                memory: doc.content,
                user_id: Some(doc.path),
                metadata: doc.metadata,
            }
        })),
        _ => Json(serde_json::json!({
            "status": "error",
            "message": "Memory not found"
        })),
    }
}

pub async fn v1_memories_update(
    Extension(workspace): Extension<WorkspaceContext>,
    Path(id): Path<String>,
    Json(payload): Json<V1AddMemoryRequest>,
) -> impl IntoResponse {
    let Some(existing) = workspace.workspace.memory.get(&id).await.ok().flatten() else {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Memory not found"
        }));
    };

    let content = if let Some(text) = payload.text {
        text
    } else if let Some(messages) = payload.messages {
        messages
            .into_iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        existing.content.clone()
    };

    let path = payload
        .user_id
        .clone()
        .unwrap_or_else(|| existing.path.clone());
    let mut metadata = existing.metadata.clone();
    if let Some(extra) = payload.metadata {
        if let (Some(target), Some(source)) = (metadata.as_object_mut(), extra.as_object()) {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        } else {
            metadata = extra;
        }
    }

    let mut namespace = payload.namespace;
    if let Some(uid) = payload.user_id {
        metadata["user_id"] = serde_json::json!(uid);
        if namespace
            .as_ref()
            .and_then(|value| value.user_id.as_ref())
            .is_none()
        {
            let mut value = namespace.unwrap_or_default();
            value.user_id = metadata
                .get("user_id")
                .and_then(|entry| entry.as_str())
                .map(|entry| entry.to_string());
            namespace = Some(value);
        }
    }

    match workspace
        .workspace
        .update_primary_memory(
            &id,
            path,
            content,
            metadata,
            Some(TypedMemoryPayload {
                kind: payload.kind,
                evidence_kind: payload.evidence_kind,
                namespace,
                provenance: payload.provenance,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(Some(updated_id)) => Json(serde_json::json!({
            "status": "ok",
            "message": "Memory updated successfully",
            "id": updated_id,
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "error",
            "message": "Memory not found"
        })),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "message": error.to_string()
        })),
    }
}

pub async fn v1_memories_delete(
    Extension(workspace): Extension<WorkspaceContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match workspace.workspace.memory.delete(&id).await {
        Ok(Some(doc)) => {
            if let Some(memory_id) = doc.id.clone().or_else(|| Some(doc.path.clone())) {
                if let Err(error) = workspace.workspace.remove_memory_entities(&memory_id).await {
                    tracing::warn!(%error, memory_id = %memory_id, "failed to remove entity graph memory index");
                }
            }
            Json(serde_json::json!({
                "status": "ok",
                "message": "Memory deleted successfully"
            }))
        }
        _ => Json(serde_json::json!({
            "status": "error",
            "message": "Memory not found"
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    use crate::{
        agents::RuntimeConfig,
        memory::file_indexer::{FileIndexer, FileIndexerConfig},
        workspace::{WorkspaceConfig, WorkspaceContext, WorkspaceRegistry, WorkspaceState},
        AppState,
    };

    fn unique_test_path(prefix: &str, suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should not be before UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}-{suffix}"))
    }

    async fn test_state() -> (AppState, WorkspaceContext) {
        let db_path = unique_test_path("xavier-v1-test", "code_graph.db");
        let code_db = Arc::new(
            code_graph::db::CodeGraphDB::new(&db_path)
                .expect("failed to create CodeGraphDB for test"),
        );
        let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
        let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));
        let workspace_registry = Arc::new(WorkspaceRegistry::new());
        let workspace = WorkspaceState::new(
            WorkspaceConfig {
                id: "test".to_string(),
                token: "test-token".to_string(),
                plan: crate::workspace::PlanTier::Personal,
                memory_backend: crate::memory::store::MemoryBackend::File,
                storage_limit_bytes: Some(10 * 1024 * 1024),
                request_limit: Some(10_000),
                request_unit_limit: Some(20_000),
                embedding_provider_mode: crate::workspace::EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: crate::workspace::SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            unique_test_path("xavier-v1-panel", "threads"),
        )
        .await
        .expect("failed to create WorkspaceState for test");
        workspace_registry
            .insert(workspace)
            .await
            .expect("failed to insert workspace into registry");
        let workspace = workspace_registry
            .authenticate("test-token")
            .await
            .expect("failed to authenticate with test token");

        (
            AppState {
                workspace_registry,
                indexer: FileIndexer::new(FileIndexerConfig::default(), Some(code_indexer.clone())),
                code_indexer,
                code_query,
                code_db,
                pattern_adapter: Arc::new(
                    crate::adapters::outbound::vec::pattern_adapter::PatternAdapter::new(),
                ),
                security_service: Arc::new(crate::app::security_service::SecurityService::new()),
            },
            workspace,
        )
    }

    fn test_router(state: AppState, workspace: WorkspaceContext) -> Router {
        Router::new()
            .route("/v1/memories", post(v1_memories_add).get(v1_memories_list))
            .route(
                "/v1/memories/{id}",
                get(v1_memories_get)
                    .put(v1_memories_update)
                    .delete(v1_memories_delete),
            )
            .route("/v1/memories/search", post(v1_memories_search))
            .layer(Extension(workspace))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_v1_memories_crud() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        // 1. Add Memory
        let add_req = Request::builder()
            .method("POST")
            .uri("/v1/memories")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "text": "test memory content",
                    "user_id": "user123",
                    "metadata": {"category": "test"}
                })
                .to_string(),
            ))
            .expect("failed to build add memory request");

        let resp = app
            .clone()
            .oneshot(add_req)
            .await
            .expect("failed to execute add memory request");
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. List Memories
        let list_req = Request::builder()
            .method("GET")
            .uri("/v1/memories?limit=10")
            .body(Body::empty())
            .expect("failed to build list memories request");
        let resp = app
            .clone()
            .oneshot(list_req)
            .await
            .expect("failed to execute list memories request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("failed to read list response body");
        let list_resp: V1MemoryListResponse =
            serde_json::from_slice(&body).expect("failed to parse list response JSON");
        assert_eq!(list_resp.memories.len(), 1);
        let memory_id = list_resp.memories[0].id.clone();

        // 3. Get Memory
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/memories/{}", memory_id))
            .body(Body::empty())
            .expect("failed to build get memory request");
        let resp = app
            .clone()
            .oneshot(get_req)
            .await
            .expect("failed to execute get memory request");
        assert_eq!(resp.status(), StatusCode::OK);

        // 4. Update Memory
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/memories/{}", memory_id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "text": "updated content",
                    "user_id": "user123"
                })
                .to_string(),
            ))
            .expect("failed to build update memory request");
        let resp = app
            .clone()
            .oneshot(update_req)
            .await
            .expect("failed to execute update memory request");
        assert_eq!(resp.status(), StatusCode::OK);

        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/memories/{}", memory_id))
            .body(Body::empty())
            .expect("failed to build get (after update) memory request");
        let resp = app
            .clone()
            .oneshot(get_req)
            .await
            .expect("failed to execute get (after update) memory request");
        let body = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("failed to read get (after update) response body");
        let payload: serde_json::Value = serde_json::from_slice(&body)
            .expect("failed to parse get (after update) response JSON");
        assert_eq!(payload["memory"]["id"].as_str(), Some(memory_id.as_str()));
        assert_eq!(
            payload["memory"]["memory"].as_str(),
            Some("updated content")
        );
        assert_eq!(payload["memory"]["metadata"]["revision"].as_u64(), Some(2));

        // 5. Search Memory
        let search_req = Request::builder()
            .method("POST")
            .uri("/v1/memories/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "updated",
                    "limit": 5
                })
                .to_string(),
            ))
            .expect("failed to build search memory request");
        let resp = app
            .clone()
            .oneshot(search_req)
            .await
            .expect("failed to execute search memory request");
        assert_eq!(resp.status(), StatusCode::OK);

        // 6. Delete Memory
        let delete_req = Request::builder()
            .method("DELETE")
            .uri(format!("/v1/memories/{}", memory_id))
            .body(Body::empty())
            .expect("failed to build delete memory request");
        let resp = app
            .oneshot(delete_req)
            .await
            .expect("failed to execute delete memory request");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_v1_memories_pagination() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        // Add 5 memories
        for i in 0..5 {
            let add_req = Request::builder()
                .method("POST")
                .uri("/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "text": format!("memory {}", i),
                        "user_id": "user123"
                    })
                    .to_string(),
                ))
                .expect("failed to build add (pagination) memory request");
            app.clone()
                .oneshot(add_req)
                .await
                .expect("failed to execute add (pagination) memory request");
        }

        // Test pagination: limit=2, offset=1
        let list_req = Request::builder()
            .method("GET")
            .uri("/v1/memories?limit=2&offset=1")
            .body(Body::empty())
            .expect("failed to build pagination list request");
        let resp = app
            .oneshot(list_req)
            .await
            .expect("failed to execute pagination list request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("failed to read pagination response body");
        let list_resp: V1MemoryListResponse =
            serde_json::from_slice(&body).expect("failed to parse pagination response JSON");

        assert_eq!(list_resp.memories.len(), 2);
        assert_eq!(list_resp.pagination.total, 5);
        assert_eq!(list_resp.pagination.limit, 2);
        assert_eq!(list_resp.pagination.offset, 1);
    }

    #[tokio::test]
    async fn test_v1_memories_search_supports_typed_filters_and_user_namespace() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        for payload in [
            serde_json::json!({
                "text": "Decision: use typed provenance for OpenClaw bridge.",
                "user_id": "belal",
                "kind": "decision",
                "namespace": {
                    "project": "xavier",
                    "session_id": "session-typed"
                },
                "provenance": {
                    "source_app": "openclaw",
                    "source_type": "bridge_import"
                }
            }),
            serde_json::json!({
                "text": "Task: keep generic summaries secondary to specific evidence.",
                "user_id": "other-user",
                "kind": "task",
                "namespace": {
                    "project": "xavier"
                },
                "provenance": {
                    "source_app": "engram",
                    "source_type": "observation"
                }
            }),
        ] {
            let add_req = Request::builder()
                .method("POST")
                .uri("/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("failed to build add (typed) memory request");
            let resp = app
                .clone()
                .oneshot(add_req)
                .await
                .expect("failed to execute add (typed) memory request");
            assert_eq!(resp.status(), StatusCode::OK);
        }

        let search_req = Request::builder()
            .method("POST")
            .uri("/v1/memories/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "typed provenance bridge",
                    "limit": 5,
                    "filters": {
                        "kinds": ["decision"],
                        "project": "xavier",
                        "user_id": "belal",
                        "source_app": "openclaw"
                    }
                })
                .to_string(),
            ))
            .expect("failed to build search (typed) request");
        let resp = app
            .oneshot(search_req)
            .await
            .expect("failed to execute search (typed) request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("failed to read search (typed) response body");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("failed to parse search (typed) response JSON");
        let results = payload["results"]
            .as_array()
            .expect("search response 'results' should be an array");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["user_id"], "belal");
        assert_eq!(results[0]["metadata"]["kind"], "decision");
        assert_eq!(results[0]["metadata"]["namespace"]["project"], "xavier");
        assert_eq!(
            results[0]["metadata"]["provenance"]["source_app"],
            "openclaw"
        );
    }
}
