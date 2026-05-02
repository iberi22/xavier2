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

#[cfg(test)]
mod tests {
    use super::{AutoVerifier, VerificationResult};

    #[test]
    fn exact_match_scores_one() {
        let original = "Verification content with a long stable signature";

        assert_eq!(
            AutoVerifier::compute_match_score_from_text(original, original),
            1.0
        );
    }

    #[test]
    fn strong_partial_match_can_satisfy_healthy_threshold() {
        let original = "Verification content with a long stable signature and original suffix";
        let retrieved = "Verification content with a long stable signature and changed suffix";
        let match_score = AutoVerifier::compute_match_score_from_text(retrieved, original);

        assert_eq!(match_score, 0.9);
        assert!(VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score,
            latency_ms: 10,
        }
        .is_healthy());
    }

    #[test]
    fn moderate_partial_match_scores_below_healthy_threshold() {
        let original = "Verification content with a long stable signature and original suffix";
        let retrieved = "Verification content with a long stable";

        assert_eq!(
            AutoVerifier::compute_match_score_from_text(retrieved, original),
            0.7
        );
    }

    #[test]
    fn weak_length_only_partial_scores_low() {
        let original = "Verification content with a long stable signature and original suffix";
        let retrieved = "Different content with enough length to be weakly related";

        assert_eq!(
            AutoVerifier::compute_match_score_from_text(retrieved, original),
            0.3
        );
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

        let total_latency_ms = start.elapsed().as_millis() as u64;

        let result = VerificationResult {
            path: path.to_string(),
            save_ok,
            retrieve_ok,
            match_score,
            latency_ms: total_latency_ms,
        };

        info!(?result, "verification complete");
        Ok(result)
    }

    fn compute_match_score_from_text(retrieved: &str, original: &str) -> f32 {
        if retrieved.is_empty() || original.is_empty() {
            return 0.0;
        }

        // Exact match
        if retrieved == original {
            return 1.0;
        }

        let orig_len = original.len();
        let retr_len = retrieved.len();

        // Check length constraint: retrieved must be > 50% of original
        let len_ratio = retr_len as f32 / orig_len as f32;
        if len_ratio < 0.5 {
            return 0.0;
        }

        // Check partial hash match (first 32 chars as signature)
        let sig_len = std::cmp::min(32, orig_len);
        let orig_sig = &original[..sig_len];

        if retrieved.starts_with(orig_sig) || retrieved.contains(orig_sig) {
            if len_ratio > 0.8 {
                return 0.9;
            }

            return 0.7;
        }

        // Fallback: simple length-based partial score
        if len_ratio >= 0.5 {
            return 0.3;
        }

        0.0
    }

    async fn compute_match_score(resp: reqwest::Response, original: &str) -> f32 {
        match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                let results = json.get("results").and_then(|r| r.as_array());
                match results {
                    Some(arr) if !arr.is_empty() => {
                        // Try to extract content from first result
                        let first = &arr[0];
                        let retrieved = first
                            .get("content")
                            .or_else(|| first.get("text"))
                            .or_else(|| first.get("value"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        Self::compute_match_score_from_text(retrieved, original)
                    }
                    _ => 0.0,
                }
            }
            Err(_) => 0.0,
        }
    }
}
