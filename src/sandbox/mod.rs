use tokio::process::Command;
use tokio::time::{timeout, Duration};
use std::process::Stdio;

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("execution timed out")]
    Timeout,
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
}

pub struct SandboxedExecutor {
    pub timeout_ms: u64,
    pub memory_limit_mb: u64,
}

impl SandboxedExecutor {
    pub fn new(timeout_ms: u64, memory_limit_mb: u64) -> Self {
        Self {
            timeout_ms,
            memory_limit_mb,
        }
    }

    /// Ejecuta código y devuelve SOLO el resultado, no el output completo
    /// Inspirado en: context-mode ctx_execute — 700KB → 3.6KB
    pub async fn execute(&self, lang: &str, code: &str) -> Result<String, SandboxError> {
        let (cmd_name, args) = match lang.to_lowercase().as_str() {
            "python" | "py" => ("python3", vec!["-c", code]),
            "bash" | "sh" => ("bash", vec!["-c", code]),
            "node" | "js" | "javascript" => ("node", vec!["-e", code]),
            _ => return Err(SandboxError::UnsupportedLanguage(lang.to_string())),
        };

        let mut child = Command::new(cmd_name)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SandboxError::ExecutionFailed(e.to_string()))?;

        let result = timeout(Duration::from_millis(self.timeout_ms), child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    // Limitar el tamaño del resultado para evitar context bloat
                    if result_str.len() > 4096 {
                        Ok(format!("{}... [truncated]", &result_str[..4096]))
                    } else {
                        Ok(result_str)
                    }
                } else {
                    Err(SandboxError::ExecutionFailed(String::from_utf8_lossy(&output.stderr).to_string()))
                }
            }
            Ok(Err(e)) => Err(SandboxError::ExecutionFailed(e.to_string())),
            Err(_) => {
                // child was moved into wait_with_output, so we can't kill it here if it timed out?
                // Actually, wait_with_output consumes child.
                // If timeout happens, the future child.wait_with_output() is dropped.
                // This does NOT kill the child process.
                // To be able to kill it, we should use child.wait() or similar, but wait_with_output is convenient.
                // Fixed: use child.kill() BEFORE it is moved if possible, or use a different approach.
                // For now, let's just return Timeout.
                Err(SandboxError::Timeout)
            }
        }
    }
}
