use crate::domain::pattern::{PatternCategory, VerifiedPattern};
use crate::ports::inbound::PatternDiscoverPort;
use crate::ports::outbound::StoragePort;
use async_trait::async_trait;

#[allow(dead_code)]
pub struct PatternService<S: StoragePort> {
    storage: S,
}

impl<S: StoragePort> PatternService<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: StoragePort + Send + Sync> PatternDiscoverPort for PatternService<S> {
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
