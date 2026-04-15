//! Self-Improving Agent for Xavier2
//! Analyzes performance and generates improvements

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Performance metrics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: u64,
    pub success_rate: f64,
}

/// A self-improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub id: String,
    pub improvement_type: ImprovementType,
    pub description: String,
    pub expected_impact: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementType {
    PromptOptimization,
    ParameterTuning,
    NewTool,
    ContextExpansion,
    ErrorHandling,
}

/// Self-improving agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImproveConfig {
    pub enabled: bool,
    pub analyze_interval_seconds: u64,
    pub min_improvement_threshold: f64,
    pub max_improvements_per_cycle: usize,
}

impl Default for SelfImproveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            analyze_interval_seconds: 300, // 5 minutes
            min_improvement_threshold: 0.01, // 1%
            max_improvements_per_cycle: 3,
        }
    }
}

/// Self-improving agent that learns from interactions
pub struct SelfImproveAgent {
    config: SelfImproveConfig,
    metrics: Arc<RwLock<AgentMetrics>>,
    improvements: Arc<RwLock<Vec<Improvement>>>,
}

impl SelfImproveAgent {
    pub fn new(config: SelfImproveConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(AgentMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_latency_ms: 0,
                success_rate: 0.0,
            })),
            improvements: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a request result
    pub async fn record_request(&self, success: bool, latency_ms: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;

        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update average latency
        let total = metrics.total_requests as f64;
        let current_avg = metrics.average_latency_ms as f64;
        metrics.average_latency_ms = ((current_avg * (total - 1.0)) + latency_ms as f64 / total) as u64;

        // Update success rate
        metrics.success_rate = metrics.successful_requests as f64 / total;
    }

    /// Analyze performance and generate improvements
    pub async fn analyze_performance(&self) -> Vec<Improvement> {
        let metrics = self.metrics.read().await;

        let mut improvements = Vec::new();

        // If success rate is low, suggest prompt optimization
        if metrics.success_rate < 0.8 {
            improvements.push(Improvement {
                id: ulid::Ulid::new().to_string(),
                improvement_type: ImprovementType::PromptOptimization,
                description: "Success rate below 80%. Consider optimizing prompts.".to_string(),
                expected_impact: 0.15,
                confidence: 0.8,
            });
        }

        // If latency is high, suggest optimization
        if metrics.average_latency_ms > 5000 {
            improvements.push(Improvement {
                id: ulid::Ulid::new().to_string(),
                improvement_type: ImprovementType::ParameterTuning,
                description: "High latency detected. Consider reducing max_tokens.".to_string(),
                expected_impact: 0.2,
                confidence: 0.7,
            });
        }

        // Store improvements
        let mut stored = self.improvements.write().await;
        *stored = improvements.clone();

        improvements
    }

    /// Generate specific improvements based on failure patterns
    pub async fn generate_improvements(&self, failure_patterns: Vec<String>) -> Vec<Improvement> {
        let mut improvements = Vec::new();

        for pattern in failure_patterns {
            let improvement = match pattern.as_str() {
                "timeout" => Improvement {
                    id: ulid::Ulid::new().to_string(),
                    improvement_type: ImprovementType::ParameterTuning,
                    description: "Increase timeout for long-running tasks".to_string(),
                    expected_impact: 0.1,
                    confidence: 0.9,
                },
                "invalid_response" => Improvement {
                    id: ulid::Ulid::new().to_string(),
                    improvement_type: ImprovementType::PromptOptimization,
                    description: "Improve prompt to get valid responses".to_string(),
                    expected_impact: 0.2,
                    confidence: 0.8,
                },
                "context_overflow" => Improvement {
                    id: ulid::Ulid::new().to_string(),
                    improvement_type: ImprovementType::ContextExpansion,
                    description: "Increase context window or optimize memory usage".to_string(),
                    expected_impact: 0.15,
                    confidence: 0.7,
                },
                _ => Improvement {
                    id: ulid::Ulid::new().to_string(),
                    improvement_type: ImprovementType::ErrorHandling,
                    description: format!("Handle error pattern: {}", pattern),
                    expected_impact: 0.1,
                    confidence: 0.5,
                },
            };

            improvements.push(improvement);
        }

        improvements
    }

    /// Apply an improvement
    pub async fn apply_improvement(&self, improvement: &Improvement) -> Result<(), String> {
        // In a real implementation, this would modify the agent config
        // For now, we just log it
        tracing::info!("Applying improvement: {}", improvement.description);
        Ok(())
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> AgentMetrics {
        self.metrics.read().await.clone()
    }

    /// Get pending improvements
    pub async fn get_improvements(&self) -> Vec<Improvement> {
        self.improvements.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_self_improve_agent() {
        let agent = SelfImproveAgent::new(SelfImproveConfig::default());

        // Record some requests
        agent.record_request(true, 1000).await;
        agent.record_request(true, 1500).await;
        agent.record_request(false, 500).await;

        // Check metrics
        let metrics = agent.get_metrics().await;
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);

        // Analyze performance
        let improvements = agent.analyze_performance().await;
        assert!(!improvements.is_empty());
    }
}
