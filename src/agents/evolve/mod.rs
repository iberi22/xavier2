//! Evolve Module - Autonomous Self-Improving Memory System
//!
//! Based on Karpathy's autoresearch loop pattern:
//! - Fixed time budget per experiment
//! - Single metric optimization
//! - Keep/discard based on metric improvement
//! - Crash recovery
//! - Never stop until human interrupts
//!
//! The Evolve Module autonomously improves Xavier's memory architecture by:
//! 1. Researcher: scans for new memory techniques
//! 2. Experimenter: modifies memory code with hypotheses
//! 3. Evaluator: runs benchmarks (LoCoMo, Evo-Memory)
//! 4. Reflector: analyzes results, generates new hypotheses
//! 5. Integrator: keeps winning changes, discards losers

pub mod config;
pub mod experiment;
pub mod evaluator;
pub mod integrator;
pub mod reflector;
pub mod researcher;
pub mod results;

use anyhow::Result;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub use config::EvolveConfig;
pub use results::ExperimentResult;

/// Evolve Module - Main coordinator for the autonomous improvement loop
pub struct EvolveModule {
    config: EvolveConfig,
    state: Arc<RwLock<EvolveState>>,
    researcher: researcher::Researcher,
    evaluator: evaluator::Evaluator,
    integrator: integrator::Integrator,
}

#[derive(Debug, Clone)]
pub struct EvolveState {
    pub current_tag: String,
    pub experiments_run: u64,
    pub experiments_kept: u64,
    pub experiments_discarded: u64,
    pub experiments_crashed: u64,
    pub last_metric: Option<f32>,
    pub best_metric: Option<f32>,
    pub running: bool,
}

impl Default for EvolveState {
    fn default() -> Self {
        Self {
            current_tag: current_date_tag(),
            experiments_run: 0,
            experiments_kept: 0,
            experiments_discarded: 0,
            experiments_crashed: 0,
            last_metric: None,
            best_metric: None,
            running: false,
        }
    }
}

impl EvolveModule {
    /// Create a new Evolve Module
    pub fn new(config: EvolveConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(EvolveState::default())),
            researcher: researcher::Researcher::new(),
            evaluator: evaluator::Evaluator::new(config.benchmark),
            integrator: integrator::Integrator::new(),
            config,
        }
    }

    /// Start the autonomous evolution loop
    pub async fn run(&self) -> Result<()> {
        {
            let mut state = self.state.write().await;
            if state.running {
                warn!("Evolve Module already running");
                return Ok(());
            }
            state.running = true;
            info!("🚀 Starting Evolve Module - autonomous loop");
            info!("Tag: {}", state.current_tag);
            info!("Time budget per experiment: {}s", self.config.time_budget_secs);
            info!("Metric: {}", self.config.metric);
        }

        loop {
            let should_stop = {
                let state = self.state.read().await;
                !state.running
            };
            if should_stop {
                break;
            }

            match self.run_single_experiment().await {
                Ok(result) => {
                    let mut state = self.state.write().await;
                    state.experiments_run += 1;
                    state.last_metric = Some(result.metric_value);

                    if result.metric_value < state.best_metric.unwrap_or(f32::MAX) {
                        state.best_metric = Some(result.metric_value);
                        state.experiments_kept += 1;
                        info!(
                            experiments_run = state.experiments_run,
                            experiments_kept = state.experiments_kept,
                            metric = result.metric_value,
                            "✅ Kept improvement"
                        );
                    } else {
                        state.experiments_discarded += 1;
                        info!(
                            experiments_run = state.experiments_run,
                            experiments_discarded = state.experiments_discarded,
                            "❌ Discarded - no improvement"
                        );
                    }
                }
                Err(e) => {
                    let mut state = self.state.write().await;
                    state.experiments_crashed += 1;
                    warn!(
                        experiments_crashed = state.experiments_crashed,
                        error = %e,
                        "💥 Experiment crashed"
                    );
                }
            }

            // Log results to TSV
            self.log_results().await?;
        }

        info!("🛑 Evolve Module stopped");
        Ok(())
    }

    /// Stop the evolution loop
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        state.running = false;
        info!("Stopping Evolve Module after {} experiments", state.experiments_run);
    }

    /// Run a single experiment
    async fn run_single_experiment(&self) -> Result<ExperimentResult> {
        // 1. Researcher generates hypothesis
        let hypothesis = self.researcher.generate_hypothesis().await?;

        info!(hypothesis = %hypothesis.description, "🔬 Running experiment");

        // 2. Experimenter applies changes (if any)
        let backup = self.integrator.backup_memory_modules().await?;

        let modified = self
            .integrator
            .apply_hypothesis(&hypothesis)
            .await?;

        if !modified {
            info!("No changes applied - using baseline");
            self.integrator.restore(backup).await?;
            return Ok(ExperimentResult::baseline());
        }

        // 3. Run experiment with time budget
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.time_budget_secs),
            self.evaluator.evaluate(),
        )
        .await;

        // 4. Restore or keep changes based on result
        match result {
            Ok(Ok(metric_value)) => {
                let improved = self.is_improvement(metric_value).await;
                if improved {
                    // Keep changes - commit
                    self.integrator.commit(&hypothesis).await?;
                } else {
                    // Discard - restore
                    self.integrator.restore(backup).await?;
                    self.integrator.reset_to_baseline().await?;
                }
                Ok(ExperimentResult {
                    hypothesis_id: hypothesis.id,
                    metric_value,
                    status: if improved {
                        experiment::ExperimentStatus::Kept
                    } else {
                        experiment::ExperimentStatus::Discarded
                    },
                    commit_hash: None,
                    crashed: false,
                })
            }
            Ok(Err(e)) => {
                // Evaluation failed
                self.integrator.restore(backup).await?;
                Err(e)
            }
            Err(_) => {
                // Timeout - experiment took too long
                self.integrator.restore(backup).await?;
                self.integrator.reset_to_baseline().await?;
                Err(anyhow::anyhow!("Experiment timed out after {}s", self.config.time_budget_secs))
            }
        }
    }

    /// Check if metric is an improvement (lower is better for val_bpb)
    async fn is_improvement(&self, metric: f32) -> bool {
        let state = self.state.read().await;
        match state.best_metric {
            Some(best) => metric < best,
            None => true, // First experiment is always improvement
        }
    }

    /// Log results to TSV
    async fn log_results(&self) -> Result<()> {
        let state = self.state.read().await;
        let results_path = self.config.results_path();

        let line = format!(
            "{}\t{:.6}\t{:.1}\t{}\texp_{}\n",
            current_commit_hash(),
            state.last_metric.unwrap_or(0.0),
            state.experiments_kept as f32 * 44.0, // memory_gb estimate
            match state.last_metric {
                Some(m) if m <= state.best_metric.unwrap_or(f32::MAX) => "keep",
                _ => "discard",
            },
            state.experiments_run
        );

        tokio::fs::create_dir_all(results_path.parent().unwrap()).await?;
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&results_path)
            .await?
            .write_all(line.as_bytes())
            .await?;

        Ok(())
    }

    /// Get current state
    pub async fn state(&self) -> EvolveState {
        self.state.read().await.clone()
    }
}

/// Get current date-based tag (e.g., "mar24")
fn current_date_tag() -> String {
    let now = chrono::Local::now();
    format!("{}{}", now.format("%b").to_string().to_lowercase(), now.format("%d"))
}

/// Get current git commit hash (short)
fn current_commit_hash() -> String {
    // This would be replaced with actual git command in production
    "local".to_string()
}
