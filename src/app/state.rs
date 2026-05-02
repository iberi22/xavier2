use std::sync::Arc;

use crate::ports::inbound::{
    AgentLifecyclePort, InputSecurityPort, MemoryQueryPort, TimeMetricsPort,
};

#[derive(Clone)]
pub struct AppState {
    pub memory: Arc<dyn MemoryQueryPort>,
    pub time_metrics: Arc<dyn TimeMetricsPort>,
    pub agent_registry: Arc<dyn AgentLifecyclePort>,
    pub security: Arc<dyn InputSecurityPort>,
}
