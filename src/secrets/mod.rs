use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("Secret not found: {0}")]
    NotFound(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Approval denied for operation: {0}")]
    ApprovalDenied(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

pub type SecretResult<T> = Result<T, SecretError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub key: String,
    pub value: String,
}

#[derive(Default)]
pub struct SecretsManager {
    secrets: HashMap<String, String>,
}

impl SecretsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }

    pub fn store(&mut self, key: String, value: String) -> SecretResult<()> {
        self.secrets.insert(key, value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> SecretResult<String> {
        self.secrets
            .get(key)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(key.to_string()))
    }

    pub fn delete(&mut self, key: &str) -> SecretResult<()> {
        self.secrets.remove(key);
        Ok(())
    }

    pub fn exists(&self, key: &str) -> bool {
        self.secrets.contains_key(key)
    }
}

// Lending engine
pub mod lending;

// TODO: Dead code - remove or wire secret daemon into production runtime.
#[allow(dead_code)]
pub mod daemon;
pub mod local;
pub mod openbao;
pub mod store;
#[cfg(test)]
mod tests;
