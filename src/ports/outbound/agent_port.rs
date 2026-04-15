use async_trait::async_trait;

#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    async fn run_agent(&self, prompt: &str, context: serde_json::Value) -> anyhow::Result<String>;
}
