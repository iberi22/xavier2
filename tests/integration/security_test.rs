//! Security Module Tests

#[cfg(test)]
mod security_tests {
    use xavier::security::{SecurityConfig, SecurityManager};

    #[test]
    fn test_security_config_creation() {
        let config = SecurityConfig::new();

        assert!(config.enabled);
        assert_eq!(config.encryption_algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_encode_decode() {
        let security = SecurityManager::new();

        let encoded = security.encode("secret data").expect("test assertion");
        assert_ne!(encoded, "secret data");
        assert!(encoded.starts_with("hex:"));

        let decoded = security.decode(&encoded).expect("test assertion");
        assert_eq!(decoded, "secret data");
    }

    #[test]
    fn test_hash_password() {
        let security = SecurityManager::new();

        let hash = security
            .hash_password("test_password")
            .expect("test assertion");
        assert!(security
            .verify_password("test_password", &hash)
            .expect("test assertion"));
        assert!(!security
            .verify_password("wrong_password", &hash)
            .expect("test assertion"));
    }

    #[test]
    fn test_generate_token() {
        // Token generation requires XAVIER_TOKEN_SECRET to be set
        std::env::set_var("XAVIER_TOKEN_SECRET", "test-secret-key-for-testing");
        let security = SecurityManager::new();

        let token = security.generate_token("user123").expect("test assertion");
        assert!(!token.is_empty());

        let validated = security.validate_token(&token);
        assert!(validated.is_ok());
    }
}

#[cfg(test)]
mod secrets_tests {
    use xavier::secrets::SecretsManager;

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
            .expect("test assertion");

        assert!(manager.exists("api_key"));
    }

    #[test]
    fn test_retrieve_secret() {
        let mut manager = SecretsManager::new();

        manager
            .store("test_key".to_string(), "test_value".to_string())
            .expect("test assertion");

        let retrieved = manager.get("test_key").expect("test assertion");
        assert_eq!(retrieved, "test_value");
    }

    #[test]
    fn test_delete_secret() {
        let mut manager = SecretsManager::new();

        manager
            .store("to_delete".to_string(), "value".to_string())
            .expect("test assertion");
        manager.delete("to_delete").expect("test assertion");

        assert!(!manager.exists("to_delete"));
    }

    #[test]
    #[ignore] // Requires actual secrets backend
    fn test_secrets_encryption_at_rest() {
        // Test that secrets are encrypted when stored
        todo!("Implement with actual secrets backend (Vault, AWS Secrets Manager, etc.)");
    }
}
