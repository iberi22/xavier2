use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::memory::qmd_memory::QmdMemory;
use crate::workspace::WorkspaceContext;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub save_ok: bool,
    pub latency_ms: u64,
    pub match_score: f32,  // 0.0 to 1.0
    pub content_identical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationMetrics {
    pub total_saves: u64,
    pub successful_saves: u64,
    pub save_ok_rate: f32,
    pub avg_latency_ms: f32,
    pub match_score_avg: f32,
}

static METRICS: Lazy<Arc<RwLock<HashMap<String, VerificationMetrics>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub async fn verify_save(memory: &QmdMemory, content: &str) -> VerificationResult {
    let start = Instant::now();

    // 1. Immediately retrieve with same content
    let search_results = match memory.search(content, 1).await {
        Ok(results) => results,
        Err(_) => return VerificationResult {
            save_ok: false,
            latency_ms: start.elapsed().as_millis() as u64,
            match_score: 0.0,
            content_identical: false,
        },
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    if search_results.is_empty() {
        return VerificationResult {
            save_ok: false,
            latency_ms,
            match_score: 0.0,
            content_identical: false,
        };
    }

    let retrieved = &search_results[0];
    let content_identical = retrieved.content == content;

    let match_score = calculate_similarity(content, &retrieved.content);

    VerificationResult {
        save_ok: true,
        latency_ms,
        match_score,
        content_identical,
    }
}

pub async fn process_verification(workspace_ctx: WorkspaceContext, path: String, content: String, result: VerificationResult) {
    // Avoid infinite recursion for metrics paths
    if path.starts_with("metrics/verification/") {
        return;
    }

    let workspace_id = workspace_ctx.workspace_id.clone();

    // 1. Update in-memory metrics
    let mut all_metrics = METRICS.write().await;
    let metrics = all_metrics.entry(workspace_id.clone()).or_insert_with(VerificationMetrics::default);

    metrics.total_saves += 1;
    if result.save_ok {
        metrics.successful_saves += 1;
    }

    metrics.save_ok_rate = metrics.successful_saves as f32 / metrics.total_saves as f32;
    metrics.avg_latency_ms = (metrics.avg_latency_ms * (metrics.total_saves as f32 - 1.0) + result.latency_ms as f32) / metrics.total_saves as f32;
    metrics.match_score_avg = (metrics.match_score_avg * (metrics.total_saves as f32 - 1.0) + result.match_score) / metrics.total_saves as f32;

    let current_metrics = metrics.clone();
    drop(all_metrics);

    // 2. Log result to feedback/xavier2/ (Async I/O)
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let log_dir = std::path::Path::new("feedback/xavier2");

    if let Err(e) = tokio::fs::create_dir_all(log_dir).await {
        tracing::error!("Failed to create feedback directory: {}", e);
    } else {
        let log_file = log_dir.join(format!("verification-{}-{}.json", workspace_id, timestamp));
        let log_data = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "content_hash": crate::utils::crypto::sha256_hex(content.as_bytes()),
            "result": result,
            "metrics_snapshot": current_metrics
        });
        if let Ok(content_str) = serde_json::to_string_pretty(&log_data) {
            if let Err(e) = tokio::fs::write(log_file, content_str).await {
                tracing::error!("Failed to write verification log: {}", e);
            }
        }
    }

    // 3. Persist metrics in Xavier2 at metrics/verification/{date}
    let metrics_path = format!("metrics/verification/{}", date);
    let _ = workspace_ctx.workspace.memory.add_document_typed(
        metrics_path,
        serde_json::to_string(&current_metrics).unwrap_or_default(),
        serde_json::json!({
            "category": "metrics",
            "type": "verification",
            "date": date,
            "workspace_id": workspace_id
        }),
        None
    ).await;

    // 4. Alert on Degradation
    if current_metrics.save_ok_rate < 0.95 || current_metrics.match_score_avg < 0.85 {
        tracing::warn!("REPORT TO CORTEX: Performance degradation detected in workspace {}", workspace_id);
        let alert_file = log_dir.join(format!("alert-{}-{}.json", workspace_id, date));
        let alert_data = serde_json::json!({
            "date": date,
            "workspace_id": workspace_id,
            "message": "Performance degradation detected",
            "metrics": current_metrics
        });
        if let Ok(content_str) = serde_json::to_string_pretty(&alert_data) {
            let _ = tokio::fs::write(alert_file, content_str).await;
        }
    }
}

fn calculate_similarity(original: &str, retrieved: &str) -> f32 {
    if original == retrieved {
        return 1.0;
    }
    if original.is_empty() || retrieved.is_empty() {
        return 0.0;
    }

    let original_words: std::collections::HashSet<_> = original.split_whitespace().collect();
    let retrieved_words: std::collections::HashSet<_> = retrieved.split_whitespace().collect();

    let intersection = original_words.intersection(&retrieved_words).count();
    let union = original_words.union(&retrieved_words).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f32 / union as f32
}
