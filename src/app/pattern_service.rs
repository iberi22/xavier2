use crate::domain::pattern::{PatternCategory, VerifiedPattern};
use crate::ports::inbound::PatternDiscoverPort;
use crate::memory::surreal_store::MemoryStore;
use async_trait::async_trait;
use std::sync::Arc;

#[allow(dead_code)]
pub struct PatternService {
    storage: Arc<dyn MemoryStore>,
}

impl PatternService {
    pub fn new(storage: Arc<dyn MemoryStore>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl PatternDiscoverPort for PatternService {
    async fn discover(&self, pattern: VerifiedPattern) -> anyhow::Result<String> {
        let _ = pattern;
        todo!()
    }

    async fn query(
        &self,
        project: &str,
        category: Option<PatternCategory>,
        min_confidence: f32,
    ) -> anyhow::Result<Vec<VerifiedPattern>> {
        let _ = project;
        let _ = category;
        let _ = min_confidence;
        todo!()
    }

    async fn verify(&self, id: &str, verified: bool) -> anyhow::Result<()> {
        let _ = id;
        let _ = verified;
        todo!()
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        let _ = id;
        todo!()
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        let _ = id;
        todo!()
    }

    async fn increment_usage(&self, id: &str) -> anyhow::Result<()> {
        let _ = id;
        todo!()
    }
}
