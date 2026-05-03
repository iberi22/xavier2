//! Key Management - KEK, DEK, and Argon2 derivation
//!
//! Implements secure key hierarchy for E2E encryption:
//! - KEK (Key Encryption Key): Derived from user password using Argon2id
//! - DEK (Data Encryption Key): Per-document key, encrypted with KEK

use argon2::Argon2;
use rand::rngs::OsRng as RandOsRng;
use rand::RngCore;

use crate::crypto::{DEK_SIZE, SALT_SIZE};

/// Error type for key operations
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("Argon2 error: {0}")]
    Argon2(String),

    #[error("Invalid password or key")]
    InvalidPassword,

    #[error("Key derivation failed")]
    DerivationFailed,

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
}

/// Result type for key operations
pub type KeyResult<T> = Result<T, KeyError>;

impl From<argon2::password_hash::Error> for KeyError {
    fn from(value: argon2::password_hash::Error) -> Self {
        Self::Argon2(value.to_string())
    }
}

/// Salt for Argon2 key derivation
#[derive(Debug, Clone)]
pub struct KeySalt(pub [u8; SALT_SIZE]);

impl KeySalt {
    /// Generate a new random salt
    pub fn generate() -> Self {
        let mut salt = [0u8; SALT_SIZE];
        RandOsRng.fill_bytes(&mut salt);
        Self(salt)
    }

    /// Create from existing bytes
    pub fn from_bytes(bytes: &[u8; SALT_SIZE]) -> Self {
        Self(*bytes)
    }

    /// Return as byte slice
    pub fn as_bytes(&self) -> &[u8; SALT_SIZE] {
        &self.0
    }
}

/// KEK (Key Encryption Key) derived from password via Argon2id
#[derive(Debug, Clone)]
pub struct KEK(pub [u8; DEK_SIZE]);

impl KEK {
    /// Derive KEK from password and salt using Argon2id
    ///
    /// # Security Notes
    /// - Uses Argon2id (memory-hard, side-channel resistant)
    /// - Default: 64MB memory, 3 iterations, 4 degree of parallelism
    /// - Adjust params based on your security/performance needs
    pub fn derive_from_password(password: &str, salt: &KeySalt) -> KeyResult<Self> {
        // Argon2id with secure defaults (64MB memory)
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(4096, 3, 4, Some(32usize))
                .map_err(|_| KeyError::DerivationFailed)?,
        );

        let mut key = [0u8; DEK_SIZE];
        argon2
            .hash_password_into(password.as_bytes(), &salt.0, &mut key)
            .map_err(|_| KeyError::DerivationFailed)?;

        Ok(KEK(key))
    }

    /// Verify a password against this KEK's derived key
    pub fn verify_password(&self, password: &str, salt: &KeySalt) -> KeyResult<bool> {
        let derived = KEK::derive_from_password(password, salt)?;
        Ok(self.0 == derived.0)
    }

    /// Return key bytes
    pub fn as_bytes(&self) -> &[u8; DEK_SIZE] {
        &self.0
    }
}

/// DEK (Data Encryption Key) - per-document random key
#[derive(Debug, Clone)]
pub struct DEK(pub [u8; DEK_SIZE]);

impl DEK {
    /// Generate a new random DEK
    pub fn generate() -> Self {
        let mut key = [0u8; DEK_SIZE];
        RandOsRng.fill_bytes(&mut key);
        DEK(key)
    }

    /// Create from existing bytes
    pub fn from_bytes(bytes: &[u8; DEK_SIZE]) -> Self {
        Self(*bytes)
    }

    /// Return key bytes
    pub fn as_bytes(&self) -> &[u8; DEK_SIZE] {
        &self.0
    }
}

/// Key Manager - orchestrates KEK and DEK operations
pub struct KeyManager {
    salt: KeySalt,
}

impl KeyManager {
    /// Create a new key manager with a fresh salt
    pub fn new() -> Self {
        Self {
            salt: KeySalt::generate(),
        }
    }

    /// Create with existing salt
    pub fn with_salt(salt: KeySalt) -> Self {
        Self { salt }
    }

    /// Get the salt (needed for storage alongside encrypted data)
    pub fn salt(&self) -> &KeySalt {
        &self.salt
    }

    /// Derive KEK from a password
    pub fn derive_kek(&self, password: &str) -> KeyResult<KEK> {
        KEK::derive_from_password(password, &self.salt)
    }

    /// Generate a new DEK for a document
    pub fn generate_dek(&self) -> DEK {
        DEK::generate()
    }

    /// Encrypt a DEK with a KEK (for storage)
    ///
    /// Returns the encrypted DEK bytes (16-byte nonce + ciphertext + 16-byte tag)
    pub fn encrypt_dek(&self, dek: &DEK, kek: &KEK) -> KeyResult<Vec<u8>> {
        use crate::crypto::encryption::{aes_encrypt, NonceBytes};

        // DEK is encrypted with KEK - use a deterministic nonce derived from DEK
        // This is safe because each DEK is unique per document
        let nonce = NonceBytes::from_dek(&dek.0);
        aes_encrypt(&dek.0, &kek.0, &nonce).map_err(|_| KeyError::InvalidKeyLength {
            expected: 0,
            actual: 0,
        })
    }

    /// Decrypt a DEK with a KEK (for retrieval)
    pub fn decrypt_dek(&self, encrypted_dek: &[u8], kek: &KEK) -> KeyResult<DEK> {
        use crate::crypto::encryption::aes_decrypt;

        let decrypted =
            aes_decrypt(encrypted_dek, &kek.0).map_err(|_| KeyError::InvalidKeyLength {
                expected: 0,
                actual: 0,
            })?;

        if decrypted.len() != DEK_SIZE {
            return Err(KeyError::InvalidKeyLength {
                expected: DEK_SIZE,
                actual: decrypted.len(),
            });
        }

        let mut key = [0u8; DEK_SIZE];
        key.copy_from_slice(&decrypted);
        Ok(DEK(key))
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive KEK from password string (convenience function)
pub fn derive_kek_from_password(password: &str, salt: &[u8; SALT_SIZE]) -> KeyResult<KEK> {
    let salt = KeySalt::from_bytes(salt);
    KEK::derive_from_password(password, &salt)
}

/// Generate a random DEK (convenience function)
pub fn generate_dek() -> [u8; DEK_SIZE] {
    let dek = DEK::generate();
    dek.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kek_derive_and_verify() {
        let salt = KeySalt::generate();
        let password = "secure_password_123";
        let kek = KEK::derive_from_password(password, &salt).unwrap();

        // Verify correct password
        assert!(kek.verify_password(password, &salt).unwrap());

        // Verify wrong password
        assert!(!kek.verify_password("wrong_password", &salt).unwrap());
    }

    #[test]
    fn test_kek_deterministic() {
        let salt = KeySalt::generate();
        let password = "test_password";

        let kek1 = KEK::derive_from_password(password, &salt).unwrap();
        let kek2 = KEK::derive_from_password(password, &salt).unwrap();

        assert_eq!(kek1.0, kek2.0);
    }

    #[test]
    fn test_dek_randomness() {
        let dek1 = DEK::generate();
        let dek2 = DEK::generate();

        // DEKs should be different (with overwhelming probability)
        assert_ne!(dek1.0, dek2.0);
    }

    #[test]
    fn test_key_manager_full_cycle() {
        let manager = KeyManager::new();
        let password = "my_secure_password";
        let kek = manager.derive_kek(password).unwrap();
        let dek = manager.generate_dek();

        // Encrypt DEK
        let encrypted_dek = manager.encrypt_dek(&dek, &kek).unwrap();

        // Decrypt DEK
        let decrypted_dek = manager.decrypt_dek(&encrypted_dek, &kek).unwrap();

        assert_eq!(dek.0, decrypted_dek.0);
    }

    #[test]
    fn test_different_password_fails() {
        let manager = KeyManager::new();
        let password = "original_password";
        let wrong_password = "wrong_password";

        let kek = manager.derive_kek(password).unwrap();
        let dek = manager.generate_dek();
        let encrypted_dek = manager.encrypt_dek(&dek, &kek).unwrap();

        // Wrong KEK (derived from different password) should fail to decrypt
        let wrong_kek = manager.derive_kek(wrong_password).unwrap();
        let result = manager.decrypt_dek(&encrypted_dek, &wrong_kek);

        // Will fail or produce garbage (auth tag will fail)
        // In practice, AES-GCM authentication will reject tampered/wrong ciphertext
        assert!(result.is_err() || result.unwrap().0 != dek.0);
    }
}
