pub use crate::ports::inbound::security_port::SecureInputResult;
use async_trait::async_trait;

#[async_trait]
pub trait InputSecurityPort: Send + Sync {
    async fn process_input(&self, input: &str) -> anyhow::Result<SecureInputResult>;
    async fn process_output(&self, output: &str) -> anyhow::Result<String>;
}
