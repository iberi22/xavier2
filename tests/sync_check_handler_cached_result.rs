use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use xavier2::adapters::inbound::http::routes::sync_check_handler;
use xavier2::domain::memory::{
    EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryRecord,
};
use xavier2::ports::outbound::health_check_port::HealthStatus;
use xavier2::ports::outbound::{HealthCheckPort, StoragePort};
use xavier2::tasks::session_sync_task::SessionSyncTask;

struct MockHealthPort;

#[async_trait]
impl HealthCheckPort for MockHealthPort {
    async fn check_health(&self) -> anyhow::Result<HealthStatus> {
        Ok(HealthStatus {
            status: "ok".to_string(),
            lag_ms: 0,
            active_agents: 7,
        })
    }
}

struct MockStoragePort {
    records: Vec<MemoryRecord>,
}

#[async_trait]
impl StoragePort for MockStoragePort {
    async fn put(&self, _record: MemoryRecord) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        Ok(None)
    }

    async fn list(&self, _namespace: &str, _limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        Ok(self.records.clone())
    }

    async fn search(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        Ok(Vec::new())
    }

    async fn delete(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        Ok(None)
    }
}

fn make_session_record(seconds_ago: i64) -> MemoryRecord {
    let event_at = Utc::now() - chrono::Duration::seconds(seconds_ago);
    let indexed_at = Utc::now();
    MemoryRecord {
        id: "session-1".to_string(),
        content: serde_json::json!({
            "session_id": "session-1",
            "event_type": "message",
            "timestamp": event_at,
            "content": "test",
        })
        .to_string(),
        kind: MemoryKind::Context,
        namespace: MemoryNamespace::Session,
        provenance: MemoryProvenance {
            source: "test".to_string(),
            evidence_kind: EvidenceKind::Direct,
            confidence: 1.0,
        },
        embedding: None,
        created_at: event_at,
        updated_at: indexed_at,
    }
}

#[tokio::test]
async fn sync_check_handler_returns_cached_result_from_session_sync_task() {
    SessionSyncTask::update_metrics(0.90, 0.88, 7);

    let storage = Arc::new(MockStoragePort {
        records: vec![make_session_record(45)],
    }) as Arc<dyn StoragePort>;
    let health = Arc::new(MockHealthPort) as Arc<dyn HealthCheckPort>;
    let task = SessionSyncTask::with_storage(health, Some(storage));

    let sync_result = task.run_sync_check().await;
    let axum::Json(response) = sync_check_handler().await;

    assert_eq!(response.status, "alert");
    assert_eq!(response.active_agents, 7);
    assert_eq!(response.save_ok_rate, 0.90);
    assert_eq!(response.match_score, 0.88);
    assert_eq!(response.timestamp_ms, sync_result.timestamp_ms);
    assert!(response.lag_ms >= 40_000 && response.lag_ms <= 50_000);
    assert_eq!(response.alerts.len(), 2);
}
