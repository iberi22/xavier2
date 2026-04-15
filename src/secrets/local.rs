use crate::secrets::store::SecretStore;
use crate::secrets::SecretResult;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

pub struct LocalSecretStore {
    storage: Mutex<HashMap<String, String>>,
}

impl Default for LocalSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSecretStore {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }
}

impl SecretStore for LocalSecretStore {
    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<String>> + Send + 'a>> {
        Box::pin(async move {
            let storage = self
                .storage
                .lock()
                .map_err(|e| crate::secrets::SecretError::DatabaseError(e.to_string()))?;
            storage
                .get(key)
                .cloned()
                .ok_or_else(|| crate::secrets::SecretError::NotFound(key.to_string()))
        })
    }

    fn set<'a>(
        &'a self,
        key: &'a str,
        value: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut storage = self
                .storage
                .lock()
                .map_err(|e| crate::secrets::SecretError::DatabaseError(e.to_string()))?;
            storage.insert(key.to_string(), value.to_string());
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = SecretResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut storage = self
                .storage
                .lock()
                .map_err(|e| crate::secrets::SecretError::DatabaseError(e.to_string()))?;
            storage.remove(key);
            Ok(())
        })
    }
}
