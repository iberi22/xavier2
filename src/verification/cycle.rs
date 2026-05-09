//! Standardized Save/Retrieve Verification Cycle for SWAL Agents
//!
//! Implements the mandatory verification loop:
//! 1. SAVE → store in Xavier with path + content
//! 2. RETRIEVE → fetch back with same query
//! 3. VERIFY → compare retrieved vs saved
//! 4. IF MISMATCH → retry (up to max_retries)
//! 5. LOG → record feedback to feedback/<system>/<date>
//! 6. REPORT → return save_ok, latency_ms, match fields
//!
//! Usage:
//! ```ignore
//! let result = VerificationCycle::new(client, xavier_url, auth_token)
//!     .verify_save("projects/manteniapp/overview", "test content")
//!     .await?;
//! assert!(result.is_healthy());
//! ```

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Configuration for the verification cycle
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Maximum retry attempts on mismatch (default: 3)
    pub max_retries: u32,
    /// Minimum match score threshold (default: 0.8)
    pub min_match_score: f32,
    /// Request timeout per attempt (default: 10s)
    pub timeout: Duration,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            min_match_score: 0.8,
            timeout: Duration::from_secs(10),
        }
    }
}

/// Result of a verification cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// The path that was verified
    pub path: String,
    /// Whether the save succeeded
    pub save_ok: bool,
    /// Whether the retrieve succeeded
    pub retrieve_ok: bool,
    /// Content match score (0.0 - 1.0)
    pub match_score: f32,
    /// Total latency in milliseconds
    pub latency_ms: u64,
    /// Number of retry attempts made
    pub attempts: u32,
    /// Whether verification passed (save_ok && retrieve_ok && match_score >= threshold)
    pub healthy: bool,
    /// Error message if any step failed
    pub error: Option<String>,
}

impl VerificationResult {
    /// Returns true if all checks passed
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    /// Returns a summary string for logging
    pub fn summary(&self) -> String {
        if self.healthy {
            format!(
                "✅ [{}] save_ok={} retrieve_ok={} match={:.2} latency={}ms attempts={}",
                self.path, self.save_ok, self.retrieve_ok, self.match_score, self.latency_ms, self.attempts
            )
        } else {
            format!(
                "❌ [{}] save_ok={} retrieve_ok={} match={:.2} latency={}ms attempts={} error={:?}",
                self.path, self.save_ok, self.retrieve_ok, self.match_score, self.latency_ms, self.attempts, self.error
            )
        }
    }
}

/// The core standardized verification cycle
pub struct VerificationCycle {
    client: Client,
    xavier_url: String,
    auth_token: String,
    config: VerificationConfig,
}

impl VerificationCycle {
    /// Create a new VerificationCycle
    pub fn new(client: Client, xavier_url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            client,
            xavier_url: xavier_url.into(),
            auth_token: auth_token.into(),
            config: VerificationConfig::default(),
        }
    }

    /// Override the default configuration
    pub fn with_config(mut self, config: VerificationConfig) -> Self {
        self.config = config;
        self
    }

    /// Run the full save→retrieve→verify cycle
    pub async fn verify_save(
        &self,
        path: &str,
        content: &str,
    ) -> Result<VerificationResult, String> {
        self.verify_save_with_kind(path, content, "verification").await
    }

    /// Run the full save→retrieve→verify cycle with a custom memory kind
    pub async fn verify_save_with_kind(
        &self,
        path: &str,
        content: &str,
        kind: &str,
    ) -> Result<VerificationResult, String> {
        let mut last_error = None;
        let mut attempts = 0u32;

        for attempt in 0..self.config.max_retries {
            attempts = attempt + 1;
            let start = Instant::now();

            // Step 1: SAVE
            let save_payload = serde_json::json!({
                "path": path,
                "content": content,
                "kind": kind,
            });

            let save_resp = self
                .client
                .post(format!("{}/memory/add", self.xavier_url))
                .header("Authorization", format!("Bearer {}", self.auth_token))
                .timeout(self.config.timeout)
                .json(&save_payload)
                .send()
                .await
                .map_err(|e| format!("save request failed: {}", e))?;

            let save_ok = save_resp.status().is_success();
            let latency_ms = start.elapsed().as_millis() as u64;

            if !save_ok {
                let err = format!(
                    "save failed with status {} on attempt {}",
                    save_resp.status(),
                    attempts
                );
                error!("{}", err);
                last_error = Some(err);
                continue;
            }

            // Step 2: RETRIEVE (with same query)
            let retrieve_payload = serde_json::json!({
                "query": content,
                "path": path,
                "limit": 5,
            });

            let retrieve_resp = self
                .client
                .post(format!("{}/memory/search", self.xavier_url))
                .header("Authorization", format!("Bearer {}", self.auth_token))
                .timeout(self.config.timeout)
                .json(&retrieve_payload)
                .send()
                .await
                .map_err(|e| format!("retrieve request failed: {}", e))?;

            let retrieve_ok = retrieve_resp.status().is_success();
            let match_score = if retrieve_ok {
                Self::compute_match_score(retrieve_resp, content).await
            } else {
                0.0
            };

            let result = VerificationResult {
                path: path.to_string(),
                save_ok,
                retrieve_ok,
                match_score,
                latency_ms,
                attempts,
                healthy: save_ok && retrieve_ok && match_score >= self.config.min_match_score,
                error: last_error.take(),
            };

            info!(result = %result.summary(), "verification cycle complete");

            // Step 3: VERIFY — if healthy, we're done
            if result.healthy {
                return Ok(result);
            }

            // Step 4: MISMATCH — retry
            let err = format!(
                "mismatch on attempt {}: match_score={:.2} < threshold={:.2}",
                attempts,
                match_score,
                self.config.min_match_score
            );
            warn!("{}", err);
            last_error = Some(err);

            // Brief backoff before retry
            tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt))).await;
        }

        // All retries exhausted
        let final_result = VerificationResult {
            path: path.to_string(),
            save_ok: false,
            retrieve_ok: false,
            match_score: 0.0,
            latency_ms: 0,
            attempts,
            healthy: false,
            error: last_error.or_else(|| Some("max retries exhausted".to_string())),
        };

        error!(result = %final_result.summary(), "verification cycle failed after {} attempts", attempts);
        Ok(final_result)
    }

    /// Compute content match score by comparing retrieved results against original content
    async fn compute_match_score(resp: reqwest::Response, original: &str) -> f32 {
        match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                let results = json.get("results").and_then(|r| r.as_array());
                match results {
                    Some(arr) if !arr.is_empty() => {
                        // Compare retrieved content against original
                        let original_lower = original.to_lowercase();
                        let original_chars: Vec<char> = original_lower.chars().filter(|c| c.is_alphanumeric()).collect();

                        let mut max_similarity = 0.0f32;

                        for result in arr {
                            let retrieved_content = result
                                .get("content")
                                .or_else(|| result.get("text"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let retrieved_chars: Vec<char> = retrieved_content
                                .to_lowercase()
                                .chars()
                                .filter(|c| c.is_alphanumeric())
                                .collect();

                            let similarity = Self::jaccard_similarity(&original_chars, &retrieved_chars);
                            max_similarity = max_similarity.max(similarity);
                        }

                        max_similarity
                    }
                    _ => 0.0,
                }
            }
            Err(_) => 0.0,
        }
    }

    /// Jaccard similarity between two character sets
    fn jaccard_similarity(a: &[char], b: &[char]) -> f32 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_set: std::collections::HashSet<_> = a.iter().collect();
        let b_set: std::collections::HashSet<_> = b.iter().collect();

        let intersection = a_set.intersection(&b_set).count();
        let union = a_set.union(&b_set).count();

        intersection as f32 / union as f32
    }

    /// Save feedback to Xavier (step 5 of the cycle)
    pub async fn save_feedback(
        &self,
        system: &str,
        result: &VerificationResult,
    ) -> Result<(), String> {
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let path = format!("feedback/{}/{}", system, date);

        let feedback_content = serde_json::to_string_pretty(&result).unwrap_or_default();

        let feedback_payload = serde_json::json!({
            "path": path,
            "content": feedback_content,
            "kind": "feedback",
        });

        let resp = self
            .client
            .post(format!("{}/memory/add", self.xavier_url))
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .timeout(self.config.timeout)
            .json(&feedback_payload)
            .send()
            .await
            .map_err(|e| format!("feedback save failed: {}", e))?;

        if resp.status().is_success() {
            info!("feedback saved to {}", path);
            Ok(())
        } else {
            Err(format!("feedback save failed with status {}", resp.status()))
        }
    }
}

/// Helper to build a VerificationCycle from environment variables
impl VerificationCycle {
    /// Create from XAVIER_URL and XAVIER_TOKEN env vars
    pub fn from_env() -> Result<Self, String> {
        let url_str = std::env::var("XAVIER_URL")
            .or_else(|_| std::env::var("XAVIER_API_URL"))
            .unwrap_or_else(|_| crate::settings::XavierSettings::current().client_base_url());

        // Validate internal URL to prevent SSRF
        let validated_url = crate::security::url_validator::validate_internal_url(&url_str)
            .map_err(|e| format!("XAVIER_URL validation failed: {}", e))?;

        let token = std::env::var("XAVIER_TOKEN")
            .or_else(|_| std::env::var("XAVIER_AUTH_TOKEN"))
            .map_err(|_| "XAVIER_TOKEN not set")?;

        Ok(Self::new(
            Client::new(),
            validated_url.as_str().trim_end_matches('/'),
            token,
        ))
    }
}

/// Convenience wrapper for agents that already have a reqwest::Client
pub async fn verify_and_report(
    client: &Client,
    xavier_url: &str,
    auth_token: &str,
    path: &str,
    content: &str,
    agent_name: &str,
) -> VerificationResult {
    let cycle = VerificationCycle::new(client.clone(), xavier_url, auth_token);
    let result = cycle.verify_save(path, content).await.unwrap_or_else(|e| {
        VerificationResult {
            path: path.to_string(),
            save_ok: false,
            retrieve_ok: false,
            match_score: 0.0,
            latency_ms: 0,
            attempts: 0,
            healthy: false,
            error: Some(e),
        }
    });

    // Auto-save feedback on failure
    if !result.healthy {
        if let Err(e) = cycle.save_feedback(agent_name, &result).await {
            error!("failed to save feedback: {}", e);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_result_is_healthy() {
        let healthy = VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score: 0.9,
            latency_ms: 150,
            attempts: 1,
            healthy: true,
            error: None,
        };
        assert!(healthy.is_healthy());

        let unhealthy = VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score: 0.5, // below 0.8 threshold
            latency_ms: 150,
            attempts: 1,
            healthy: false,
            error: None,
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn verification_result_summary() {
        let result = VerificationResult {
            path: "projects/manteni/overview".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score: 0.92,
            latency_ms: 87,
            attempts: 1,
            healthy: true,
            error: None,
        };
        let summary = result.summary();
        assert!(summary.contains("✅"));
        assert!(summary.contains("0.92"));
    }

    #[test]
    fn jaccard_similarity_basic() {
        // Same content
        let a: Vec<char> = "hello".chars().collect();
        let b: Vec<char> = "hello".chars().collect();
        assert_eq!(VerificationCycle::jaccard_similarity(&a, &b), 1.0);

        // Completely different
        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "xyz".chars().collect();
        assert_eq!(VerificationCycle::jaccard_similarity(&a, &b), 0.0);

        // Partial overlap
        let a: Vec<char> = "hello world".chars().filter(|c| c.is_alphanumeric()).collect();
        let b: Vec<char> = "hello rust".chars().filter(|c| c.is_alphanumeric()).collect();
        let sim = VerificationCycle::jaccard_similarity(&a, &b);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn verification_config_defaults() {
        let config = VerificationConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.min_match_score, 0.8);
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn verification_cycle_from_env_not_set() {
        // Should fail when env vars are not set (cleared)
        let original_url = std::env::var("XAVIER_URL");
        let original_token = std::env::var("XAVIER_TOKEN");

        // Var is not set in test environment
        if original_url.is_err() && original_token.is_err() {
            let result = VerificationCycle::from_env();
            assert!(result.is_err());
        }
    }
}
