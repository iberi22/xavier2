use crate::ports::inbound::session_port::{SessionPort, SessionEventResult};
use crate::ports::inbound::MemoryQueryPort;
use crate::session::event_mapper::map_to_panel_thread;
use crate::session::types::SessionEvent;
use crate::domain::memory::{MemoryKind, MemoryNamespace, MemoryProvenance, EvidenceKind, MemoryRecord};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SessionService {
    memory_port: Option<Arc<dyn MemoryQueryPort>>,
}

impl SessionService {
    pub fn new() -> Self {
        Self { memory_port: None }
    }

    pub fn with_memory(memory_port: Arc<dyn MemoryQueryPort>) -> Self {
        Self {
            memory_port: Some(memory_port),
        }
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionPort for SessionService {
    async fn handle_event(&self, event: SessionEvent) -> bool {
        map_to_panel_thread(event).is_some()
    }

    async fn handle_and_index_event(&self, event: SessionEvent) -> anyhow::Result<SessionEventResult> {
        let session_id = event.session_id.clone();
        let entry = match map_to_panel_thread(event.clone()) {
            Some(e) => e,
            None => return Ok(SessionEventResult {
                status: "skipped".to_string(),
                session_id,
                memory_id: None,
                mapped: false,
            }),
        };

        let content = format!(
            "[{}] {}: {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.role,
            entry.content
        );

        let record_path = format!("sessions/{}/thread", session_id);
        
        if let Some(ref memory) = self.memory_port {
            let record = MemoryRecord {
                id: String::new(),
                content,
                kind: MemoryKind::Context,
                namespace: MemoryNamespace::Session,
                provenance: MemoryProvenance {
                    source: record_path.clone(),
                    evidence_kind: EvidenceKind::Direct,
                    confidence: 1.0,
                },
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            let id = memory.add(record).await?;
            Ok(SessionEventResult {
                status: "ok".to_string(),
                session_id,
                memory_id: Some(id),
                mapped: true,
            })
        } else {
            Ok(SessionEventResult {
                status: "ok_no_index".to_string(),
                session_id,
                memory_id: None,
                mapped: true,
            })
        }
    }
}
