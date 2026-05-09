use crate::ports::inbound::verification_port::{VerificationPort, VerificationResult};
use crate::verification::auto_verifier::AutoVerifier;
use async_trait::async_trait;

pub struct VerificationService {
    client: reqwest::Client,
}

impl VerificationService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for VerificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationPort for VerificationService {
    async fn verify_save(
        &self,
        xavier_url: &str,
        auth_token: &str,
        path: &str,
        test_content: &str,
    ) -> Result<VerificationResult, String> {
        let result =
            AutoVerifier::verify_save(&self.client, xavier_url, auth_token, path, test_content)
                .await?;

        Ok(VerificationResult {
            path: result.path,
            save_ok: result.save_ok,
            retrieve_ok: result.retrieve_ok,
            match_score: result.match_score,
            latency_ms: result.latency_ms,
        })
    }
}
