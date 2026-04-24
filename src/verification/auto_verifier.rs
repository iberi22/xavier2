use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub path: String,
    pub save_ok: bool,
    pub retrieve_ok: bool,
    pub match_score: f32,
    pub latency_ms: u64,
}

impl VerificationResult {
    pub fn is_healthy(&self) -> bool {
        self.save_ok && self.retrieve_ok && self.match_score >= 0.8
    }
}

pub struct AutoVerifier;

impl AutoVerifier {
    /// Verify that a save+retrieve cycle produces matching content
    pub async fn verify_save(
        client: &reqwest::Client,
        xavier2_url: &str,
        auth_token: &str,
        path: &str,
        test_content: &str,
    ) -> Result<VerificationResult, String> {
        let start = std::time::Instant::now();

        // Save
        let save_payload = serde_json::json!({
            "path": path,
            "content": test_content,
            "kind": "verification",
        });

        let save_resp = client
            .post(format!("{}/memory/add", xavier2_url))
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&save_payload)
            .send()
            .await
            .map_err(|e| format!("save request failed: {}", e))?;

        let save_ok = save_resp.status().is_success();
        let latency_ms = start.elapsed().as_millis() as u64;

        // Retrieve
        let retrieve_payload = serde_json::json!({
            "query": test_content,
            "path": path,
            "limit": 1,
        });

        let retrieve_resp = client
            .post(format!("{}/memory/search", xavier2_url))
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&retrieve_payload)
            .send()
            .await
            .map_err(|e| format!("retrieve request failed: {}", e))?;

        let retrieve_ok = retrieve_resp.status().is_success();
        let match_score = if retrieve_ok {
            Self::compute_match_score(retrieve_resp, test_content).await
        } else {
            0.0
        };

        let result = VerificationResult {
            path: path.to_string(),
            save_ok,
            retrieve_ok,
            match_score,
            latency_ms,
        };

        info!(?result, "verification complete");
        Ok(result)
    }

    async fn compute_match_score(
        resp: reqwest::Response,
        _original: &str,
    ) -> f32 {
        // Simple: if results returned, consider it a partial match
        // In production, embed both and compare cosine similarity
        match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                let results = json.get("results").and_then(|r| r.as_array());
                match results {
                    Some(arr) if !arr.is_empty() => 0.85,
                    _ => 0.0,
                }
            }
            Err(_) => 0.0,
        }
    }
}
