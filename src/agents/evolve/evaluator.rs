//! Evaluator Agent - Runs benchmarks to measure improvement

use crate::agents::evolve::config::BenchmarkType;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use tracing::info;

/// Evaluator - Measures the metric for the current implementation
pub struct Evaluator {
    benchmark: BenchmarkType,
}

impl Evaluator {
    pub fn new(benchmark: BenchmarkType) -> Self {
        Self { benchmark }
    }

    /// Evaluate the current implementation
    pub async fn evaluate(&self) -> Result<f32> {
        match self.benchmark {
            BenchmarkType::Locomo => self.run_locomo_benchmark().await,
            BenchmarkType::EvoMemory => self.run_evomemory_benchmark().await,
            BenchmarkType::Custom => self.run_custom_benchmark().await,
        }
    }

    async fn run_locomo_benchmark(&self) -> Result<f32> {
        info!("Running LoCoMo benchmark...");
        let output_dir = unique_benchmark_dir("locomo");
        run_benchmark_script(
            "scripts/benchmarks/run_locomo_benchmark.py",
            &[
                "--output-dir",
                output_dir.to_string_lossy().as_ref(),
                "--sample-limit",
                "1",
                "--question-limit",
                "2",
                "--mode",
                "assisted",
            ],
        )?;
        let summary = load_summary(&output_dir)?;
        let score = summary["metrics"]["overall"]["token_f1"]
            .as_f64()
            .or_else(|| {
                summary["modes"]["assisted"]["metrics"]["overall"]["token_f1"].as_f64()
            })
            .ok_or_else(|| anyhow!("LoCoMo summary missing token_f1"))? as f32;

        info!(score = score, "LoCoMo benchmark complete");
        Ok(score)
    }

    async fn run_evomemory_benchmark(&self) -> Result<f32> {
        info!("Running Evo-Memory benchmark...");
        let output_dir = unique_benchmark_dir("internal-memory");
        run_benchmark_script(
            "scripts/benchmarks/run_internal_memory_benchmark.py",
            &["--output-dir", output_dir.to_string_lossy().as_ref()],
        )?;
        let summary = load_summary(&output_dir)?;
        let score = summary["accuracy"]
            .as_f64()
            .ok_or_else(|| anyhow!("internal benchmark summary missing accuracy"))?
            as f32;

        info!(score = score, "Evo-Memory benchmark complete");
        Ok(score)
    }

    async fn run_custom_benchmark(&self) -> Result<f32> {
        info!("Running custom benchmark...");
        self.run_evomemory_benchmark().await
    }
}

fn unique_benchmark_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "xavier2-evaluator-{prefix}-{}",
        uuid::Uuid::new_v4()
    ))
}

fn run_benchmark_script(script: &str, args: &[&str]) -> Result<()> {
    #[cfg(not(feature = "bench-runners"))]
    {
        let _ = (script, args);
        return Err(anyhow!(
            "benchmark runners disabled; rebuild with --features bench-runners or run benchmarks in CI/admin"
        ));
    }

    #[cfg(feature = "bench-runners")]
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let script_path = root.join(script);
        let status = std::process::Command::new("python")
            .arg(script_path)
            .args(args)
            .current_dir(&root)
            .status()
            .context("failed to start benchmark runner")?;

        if !status.success() {
            return Err(anyhow!("benchmark runner failed with status {status}"));
        }

        Ok(())
    }
}

fn load_summary(output_dir: &PathBuf) -> Result<serde_json::Value> {
    let path = output_dir.join("summary.json");
    let payload = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read benchmark summary {}", path.display()))?;
    serde_json::from_str(&payload)
        .with_context(|| format!("failed to parse benchmark summary {}", path.display()))
}
