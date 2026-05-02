use std::path::{Path, PathBuf};

use axum::{
    extract::Path as AxumPath,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;

use crate::{
    agents::ui_render::UiRenderAgent,
    memory::session_store::{PanelMessage, SessionStore, ThreadDetail, ThreadSummary},
    workspace::WorkspaceContext,
};

const PANEL_BUILD_DIR: &str = "panel-ui/build";

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PanelChatRequest {
    pub thread_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PanelChatResponse {
    pub thread: ThreadSummary,
    pub messages: Vec<PanelMessage>,
}

pub async fn panel_index() -> impl IntoResponse {
    match tokio::fs::read_to_string(panel_build_path("index.html")).await {
        Ok(contents) => Html(contents).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Panel assets are missing. Build the panel-ui frontend first.",
        )
            .into_response(),
    }
}

pub async fn panel_asset(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    let asset_path = panel_build_path(&format!("assets/{path}"));
    match tokio::fs::read(&asset_path).await {
        Ok(bytes) => asset_response(bytes, asset_content_type(&asset_path)),
        Err(_) => (StatusCode::NOT_FOUND, "Asset not found").into_response(),
    }
}

pub async fn list_threads(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    Json(workspace.workspace.panel_store.list_threads().await)
}

pub async fn create_thread(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<CreateThreadRequest>,
) -> impl IntoResponse {
    let title_hint = payload
        .title
        .or(payload.message)
        .unwrap_or_else(|| "New Thread".to_string());

    match workspace
        .workspace
        .panel_store
        .create_thread(&title_hint)
        .await
    {
        Ok(thread) => Json(ThreadSummary::from(&thread)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_thread(
    Extension(workspace): Extension<WorkspaceContext>,
    AxumPath(thread_id): AxumPath<String>,
) -> impl IntoResponse {
    match workspace.workspace.panel_store.get_thread(&thread_id).await {
        Some(thread) => Json(ThreadDetail::from_thread(thread)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "thread not found" })),
        )
            .into_response(),
    }
}

pub async fn delete_thread(
    Extension(workspace): Extension<WorkspaceContext>,
    AxumPath(thread_id): AxumPath<String>,
) -> impl IntoResponse {
    match workspace
        .workspace
        .panel_store
        .delete_thread(&thread_id)
        .await
    {
        Ok(true) => Json(json!({ "deleted": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "thread not found" })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn process_chat(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<PanelChatRequest>,
) -> impl IntoResponse {
    match process_chat_inner(
        &workspace.workspace.panel_store,
        &workspace.workspace.runtime,
        &workspace,
        payload,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn process_chat_inner(
    store: &SessionStore,
    runtime: &std::sync::Arc<crate::agents::AgentRuntime>,
    workspace: &WorkspaceContext,
    payload: PanelChatRequest,
) -> anyhow::Result<PanelChatResponse> {
    let thread = match payload.thread_id {
        Some(thread_id) => match store.get_thread(&thread_id).await {
            Some(thread) => thread,
            None => store.create_thread(&payload.message).await?,
        },
        None => store.create_thread(&payload.message).await?,
    };

    let user_message = PanelMessage {
        id: Ulid::new().to_string(),
        role: "user".to_string(),
        plain_text: payload.message.clone(),
        openui_lang: None,
        created_at: Utc::now(),
        metadata: json!({}),
    };
    store.append_message(&thread.id, user_message).await?;

    let trace = runtime
        .run_with_trace(&payload.message, Some(thread.id.clone()), None)
        .await?;
    workspace
        .workspace
        .record_optimization(
            trace.optimization.route_category,
            trace.optimization.semantic_cache_hit,
            trace.optimization.llm_used,
            trace.optimization.model.as_deref(),
        )
        .await?;
    let ui_render = UiRenderAgent::new().render(&trace);
    let assistant_message = PanelMessage {
        id: Ulid::new().to_string(),
        role: "assistant".to_string(),
        plain_text: ui_render.plain_text,
        openui_lang: Some(ui_render.openui_lang),
        created_at: Utc::now(),
        metadata: json!({
            "confidence": trace.agent.confidence,
            "timings": trace.agent.system_timings,
            "components": ui_render.components,
            "rules": ui_render.rules_applied,
            "documents": trace.retrieval.total_results,
            "evidence": trace.reasoning.supporting_evidence.len(),
            "optimization": trace.optimization,
        }),
    };
    let updated = store.append_message(&thread.id, assistant_message).await?;
    workspace
        .workspace
        .record_session_exchange(&thread.id, "panel", &payload.message, &trace.agent.response)
        .await?;

    Ok(PanelChatResponse {
        thread: ThreadSummary::from(&updated),
        messages: updated.messages,
    })
}

fn panel_build_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(PANEL_BUILD_DIR)
        .join(relative)
}

fn asset_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn asset_response(bytes: Vec<u8>, content_type: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(axum::body::Body::from(bytes))
        .unwrap()
}

trait ThreadDetailExt {
    fn from_thread(thread: crate::memory::session_store::PanelThread) -> ThreadDetail;
}

impl ThreadDetailExt for ThreadDetail {
    fn from_thread(thread: crate::memory::session_store::PanelThread) -> ThreadDetail {
        SessionStore::detail_from_thread(thread)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agents::RuntimeConfig,
        memory::file_indexer::{FileIndexer, FileIndexerConfig},
        ports::inbound::NoopTimeMetricsPort,
        workspace::{
            EmbeddingProviderMode, PlanTier, SyncPolicy, WorkspaceConfig, WorkspaceContext,
            WorkspaceRegistry, WorkspaceState,
        },
        coordination::SimpleAgentRegistry,
        AppState,
    };
    use axum::{
        body::{to_bytes, Body},
        http::Request,
        routing::{get, post},
        Router,
    };
    use std::sync::Arc;
    use tower::util::ServiceExt;

    async fn test_state() -> (AppState, WorkspaceContext) {
        let db_path = std::env::temp_dir().join(format!("xavier2-panel-{}.db", Ulid::new()));
        let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&db_path).unwrap());
        let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
        let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));
        let workspace_registry = Arc::new(WorkspaceRegistry::new());
        let workspace = WorkspaceState::new(
            WorkspaceConfig {
                id: "panel-test".to_string(),
                token: "panel-token".to_string(),
                plan: PlanTier::Personal,
                memory_backend: crate::memory::surreal_store::MemoryBackend::File,
                storage_limit_bytes: Some(10 * 1024 * 1024),
                request_limit: Some(10_000),
                request_unit_limit: Some(20_000),
                embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            std::env::temp_dir().join(format!("xavier2-panel-store-{}", Ulid::new())),
        )
        .await
        .unwrap();
        workspace_registry.insert(workspace).await.unwrap();
        let workspace = workspace_registry
            .authenticate("panel-token")
            .await
            .unwrap();

        (
            AppState {
                workspace_id: "panel-test".to_string(),
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
            .route("/panel/api/threads", get(list_threads).post(create_thread))
            .route(
                "/panel/api/threads/{thread_id}",
                get(get_thread).delete(delete_thread),
            )
            .route("/panel/api/chat", post(process_chat))
            .layer(Extension(workspace))
            .with_state(state)
    }

    #[tokio::test]
    async fn creates_and_fetches_threads_via_http() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);
        let create_request = Request::builder()
            .method("POST")
            .uri("/panel/api/threads")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Panel Thread"}"#))
            .unwrap();

        let create_response = app.clone().oneshot(create_request).await.unwrap();
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: ThreadSummary = serde_json::from_slice(&create_body).unwrap();

        let get_request = Request::builder()
            .method("GET")
            .uri(format!("/panel/api/threads/{}", summary.id))
            .body(Body::empty())
            .unwrap();
        let get_response = app.oneshot(get_request).await.unwrap();
        let get_body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: ThreadDetail = serde_json::from_slice(&get_body).unwrap();
        assert_eq!(detail.thread.id, summary.id);
    }

    #[tokio::test]
    async fn chat_persists_assistant_ui_message() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);
        let request = Request::builder()
            .method("POST")
            .uri("/panel/api/chat")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"Explain xavier2 memory"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: PanelChatResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload.messages.len(), 2);
        assert_eq!(payload.messages[1].role, "assistant");
        assert!(payload.messages[1].openui_lang.is_some());
    }
}
