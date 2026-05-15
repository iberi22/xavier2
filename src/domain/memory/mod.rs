//! Domain models for memory operations.
//!
//! These types define the core domain contract for memory querying, storage,
//! and time metrics. Ports in `crate::ports::inbound` reference these domain
//! types rather than infrastructure types, preserving hexagonal architecture DIP.

use serde::{Deserialize, Serialize};

/// Re-export the canonical MemoryQueryFilters from memory::schema for now,
/// as the schema is the authoritative definition.
pub mod belief;

pub use belief::{BeliefEdge, BeliefNode};
pub use crate::memory::schema::MemoryQueryFilters;
pub use crate::memory::store::MemoryRecord;

pub mod graph;

/// Core TimeMetric domain value — NOT a DTO.
/// Used by the TimeMetrics inbound port to decouple from HTTP DTOs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeMetric {
    pub metric_type: String,
    pub agent_id: String,
    pub task_id: Option<String>,
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
    pub status: String,
    pub error_message: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tokens_used: Option<u64>,
    pub task_category: Option<String>,
    pub metadata: serde_json::Value,
}

impl From<TimeMetric> for crate::adapters::inbound::http::dto::TimeMetricDto {
    fn from(m: TimeMetric) -> Self {
        Self {
            metric_type: m.metric_type,
            agent_id: m.agent_id,
            task_id: m.task_id,
            started_at: m.started_at,
            completed_at: m.completed_at,
            duration_ms: m.duration_ms,
            status: m.status,
            error_message: m.error_message,
            provider: m.provider,
            model: m.model,
            tokens_used: m.tokens_used,
            task_category: m.task_category,
            metadata: m.metadata,
        }
    }
}

impl From<crate::adapters::inbound::http::dto::TimeMetricDto> for TimeMetric {
    fn from(dto: crate::adapters::inbound::http::dto::TimeMetricDto) -> Self {
        Self {
            metric_type: dto.metric_type,
            agent_id: dto.agent_id,
            task_id: dto.task_id,
            started_at: dto.started_at,
            completed_at: dto.completed_at,
            duration_ms: dto.duration_ms,
            status: dto.status,
            error_message: dto.error_message,
            provider: dto.provider,
            model: dto.model,
            tokens_used: dto.tokens_used,
            task_category: dto.task_category,
            metadata: dto.metadata,
        }
    }
}
