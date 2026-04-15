//! Security Module Tests

#[cfg(test)]
mod security_tests {
    use xavier2::security::{SecurityConfig, SecurityManager};

    #[test]
    fn test_security_config_creation() {
        let config = SecurityConfig::new();

        assert!(config.enabled);
        assert_eq!(config.encryption_algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_encryption() {
        let security = SecurityManager::new();

        let encrypted = security.encrypt("secret data").unwrap();
        assert_ne!(encrypted, "secret data");

        let decrypted = security.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "secret data");
    }

    #[test]
    fn test_hash_password() {
        let security = SecurityManager::new();

        let hash = security.hash_password("test_password").unwrap();
        assert!(security.verify_password("test_password", &hash).unwrap());
        assert!(!security.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_generate_token() {
        let security = SecurityManager::new();

        let token = security.generate_token("user123").unwrap();
        assert!(!token.is_empty());

        let validated = security.validate_token(&token);
        assert!(validated.is_ok());
    }
}

#[cfg(test)]
mod secrets_tests {
    use xavier2::secrets::SecretsManager;

    #[test]
    fn test_secrets_manager_creation() {
        let manager = SecretsManager::new();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_store_secret() {
        let mut manager = SecretsManager::new();

        manager
            .store("api_key".to_string(), "secret_value".to_string())
            .unwrap();

        assert!(manager.exists("api_key"));
    }

    #[test]
    fn test_retrieve_secret() {
        let mut manager = SecretsManager::new();

        manager
            .store("test_key".to_string(), "test_value".to_string())
            .unwrap();

        let retrieved = manager.get("test_key").unwrap();
        assert_eq!(retrieved, "test_value");
    }

    #[test]
    fn test_delete_secret() {
        let mut manager = SecretsManager::new();

        manager
            .store("to_delete".to_string(), "value".to_string())
            .unwrap();
        manager.delete("to_delete").unwrap();

        assert!(!manager.exists("to_delete"));
    }

    #[test]
    #[ignore] // Requires actual secrets backend
    fn test_secrets_encryption_at_rest() {
        // Test that secrets are encrypted when stored
        todo!("Implement with actual secrets backend (Vault, AWS Secrets Manager, etc.)");
    }
}
