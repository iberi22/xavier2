use crate::secrets::store::SecretStore;
use crate::secrets::{SecretError, SecretResult};
use std::future::Future;
use std::pin::Pin;

pub struct OpenBaoSecretStore {
    address: String,
    token: String,
}

impl OpenBaoSecretStore {
    pub fn new(address: &str, token: &str) -> Self {
        Self {
            address: address.to_string(),
            token: token.to_string(),
        }
    }
}

impl SecretStore for OpenBaoSecretStore {
    fn get<'a>(
        &'a self,
        _key: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<String>> + Send + 'a>> {
        Box::pin(async {
            Err(SecretError::ProviderError(
                "OpenBao provider not yet fully implemented".to_string(),
            ))
        })
    }

    fn set<'a>(
        &'a self,
        _key: &'a str,
        _value: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<()>> + Send + 'a>> {
        Box::pin(async {
            Err(SecretError::ProviderError(
                "OpenBao provider not yet fully implemented".to_string(),
            ))
        })
    }

    fn delete<'a>(
        &'a self,
        _key: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<()>> + Send + 'a>> {
        Box::pin(async {
            Err(SecretError::ProviderError(
                "OpenBao provider not yet fully implemented".to_string(),
            ))
        })
    }
}
