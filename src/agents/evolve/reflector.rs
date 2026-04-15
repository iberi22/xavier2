//! Reflector Agent - Analyzes results and generates insights for next experiments

use crate::agents::evolve::results::ResultsLogEntry;
use anyhow::Result;
use tracing::info;

/// Reflector - Analyzes experiment results and generates insights
pub struct Reflector {}

impl Reflector {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze results and generate insights
    pub async fn analyze(&self, results: &[ResultsLogEntry]) -> Result<Insights> {
        if results.is_empty() {
            return Ok(Insights::default());
        }

        // Find best result
        let best = results
            .iter()
            .filter(|r| r.status == "keep")
            .max_by(|a, b| a.metric.partial_cmp(&b.metric).unwrap_or(std::cmp::Ordering::Equal));

        // Count improvement rate
        let total = results.len() as f32;
        let kept = results.iter().filter(|r| r.status == "keep").count() as f32;
        let improvement_rate = kept / total;

        let insights = Insights {
            best_metric: best.map(|r| r.metric),
            best_description: best.map(|r| r.description.clone()),
            improvement_rate,
            suggestions: self.generate_suggestions(results),
        };

        info!(
            best_metric = insights.best_metric,
            improvement_rate = insights.improvement_rate,
            suggestions = ?insights.suggestions,
            "📊 Analysis complete"
        );

        Ok(insights)
    }

    /// Generate suggestions for next experiments
    fn generate_suggestions(&self, results: &[ResultsLogEntry]) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Analyze patterns
        let keep_count = results.iter().filter(|r| r.status == "keep").count();
        let discard_count = results.iter().filter(|r| r.status == "discard").count();

        if discard_count > keep_count * 2 {
            suggestions.push("Consider simpler changes - many complex changes failed".to_string());
        }

        // Check for successful patterns
        if results
            .iter()
            .any(|r| r.description.contains("cache") && r.status == "keep")
        {
            suggestions.push("Cache-related changes show promise - try more cache optimizations".to_string());
        }

        // Always suggest simplification if we haven't tried it
        suggestions.push("Consider simplification: removing code that provides equal results is a win".to_string());

        suggestions
    }
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new()
    }
}

/// Insights from analysis
#[derive(Debug, Clone, Default)]
pub struct Insights {
    /// Best metric achieved
    pub best_metric: Option<f32>,
    /// Description of best experiment
    pub best_description: Option<String>,
    /// Rate of improvements (kept / total)
    pub improvement_rate: f32,
    /// Suggestions for next experiments
    pub suggestions: Vec<String>,
}
