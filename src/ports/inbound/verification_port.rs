use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub path: String,
    pub save_ok: bool,
    pub retrieve_ok: bool,
    pub match_score: f32,
    pub latency_ms: u64,
}

#[async_trait]
pub trait VerificationPort: Send + Sync {
    async fn verify_save(
        &self,
        xavier2_url: &str,
        auth_token: &str,
        path: &str,
        test_content: &str,
    ) -> Result<VerificationResult, String>;
}
