use crate::domain::security::{ImpactReport, RiskLevel};
use crate::ports::inbound::ChangeControlPort;
use async_trait::async_trait;
use code_graph::query::QueryEngine;
use std::sync::Arc;

pub struct ChangeControlService {
    code_graph: Arc<QueryEngine>,
}

impl ChangeControlService {
    pub fn new(code_graph: Arc<QueryEngine>) -> Self {
        Self { code_graph }
    }
}

#[async_trait]
impl ChangeControlPort for ChangeControlService {
    async fn calculate_impact(&self, patterns: &[String]) -> anyhow::Result<ImpactReport> {
        let mut symbols_affected = 0;
        let mut dependent_files = std::collections::HashSet::new();
        let mut contracts_affected = std::collections::HashSet::new();

        for pattern in patterns {
            // 1. Get symbols from code-graph
            let symbols = self.code_graph.find_symbols_in_file(pattern)?;
            symbols_affected += symbols.len();

            // 2. Find reverse dependencies (who depends on this file)
            let deps = self.code_graph.find_reverse_dependencies(pattern)?;
            for dep in deps {
                dependent_files.insert(dep);
            }

            // Identify contracts affected (e.g. if we modify a trait or a public struct)
            for symbol in symbols {
                if let code_graph::types::SymbolKind::Trait | code_graph::types::SymbolKind::Struct =
                    symbol.kind
                {
                    contracts_affected.insert(symbol.name);
                }
            }
        }

        // 3. Calculate risk_score (0.0-1.0)
        let mut score: f32 = 0.0;

        // Simple heuristic:
        score += (symbols_affected as f32 * 0.05).min(0.4);
        score += (dependent_files.len() as f32 * 0.1).min(0.4);

        // 4. Identify critical_files from policy
        let critical_files = ["Cargo.toml", "src/lib.rs", "src/main.rs"];
        for pattern in patterns {
            if critical_files.iter().any(|&f| pattern == f || pattern.ends_with(&format!("/{}", f))) {
                score += 0.4;
            }
        }

        score = score.min(1.0);

        // 5. Map risk_level
        let risk_level = if score >= 0.8 {
            RiskLevel::Critical
        } else if score >= 0.5 {
            RiskLevel::High
        } else if score > 0.2 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let recommendation = match risk_level {
            RiskLevel::Critical => "requires approval - critical impact".to_string(),
            RiskLevel::High => "consider splitting task - high impact".to_string(),
            RiskLevel::Medium => "proceed with caution - medium impact".to_string(),
            RiskLevel::Low => "safe to proceed".to_string(),
        };

        Ok(ImpactReport {
            score,
            symbols_affected,
            dependent_files: dependent_files.into_iter().collect(),
            contracts_affected: contracts_affected.into_iter().collect(),
            risk_level,
            recommendation,
        })
    }
}
