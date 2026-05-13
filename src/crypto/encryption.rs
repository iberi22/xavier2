//! AES-256-GCM Encryption for Xavier E2E
//!
//! Provides symmetric encryption using AES-256 in GCM mode (authenticated encryption).
//! Each encryption operation generates a fresh nonce (IV).

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::crypto::NONCE_SIZE;

/// Size of AES-256 key in bytes
const AES_KEY_SIZE: usize = 32;

/// Size of authentication tag in bytes (included in ciphertext)
const TAG_SIZE: usize = 16;

/// Nonce (IV) for AES-256-GCM - wraps a 12-byte nonce
#[derive(Debug, Clone)]
pub struct NonceBytes(pub [u8; NONCE_SIZE]);

impl NonceBytes {
    /// Generate a random nonce
    pub fn generate() -> Self {
        let mut nonce = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce);
        Self(nonce)
    }

    /// Create from DEK bytes (for encrypting DEK with KEK)
    /// Safe because each DEK is unique - nonce is derived deterministically
    pub fn from_dek(dek: &[u8; 32]) -> Self {
        let mut nonce = [0u8; NONCE_SIZE];
        // Use first NONCE_SIZE bytes of DEK as nonce
        nonce.copy_from_slice(&dek[..NONCE_SIZE]);
        Self(nonce)
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; NONCE_SIZE]) -> Self {
        Self(*bytes)
    }

    /// Return as byte slice
    pub fn as_bytes(&self) -> &[u8; NONCE_SIZE] {
        &self.0
    }
}

/// Encrypted blob containing all data needed for decryption
#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    /// The encrypted data (ciphertext + auth tag)
    pub ciphertext: Vec<u8>,
    /// Nonce/IV used for encryption
    pub nonce: Vec<u8>,
}

impl EncryptedBlob {
    /// Serialize to bytes: [nonce (12 bytes) || ciphertext]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(NONCE_SIZE + self.ciphertext.len());
        result.extend_from_slice(&self.nonce);
        result.extend_from_slice(&self.ciphertext);
        result
    }

    /// Deserialize from bytes: [nonce (12 bytes) || ciphertext]
    pub fn from_bytes(data: &[u8]) -> Result<Self, EncryptionError> {
        if data.len() < NONCE_SIZE {
            return Err(EncryptionError::InvalidNonce);
        }

        let nonce = data[..NONCE_SIZE].to_vec();
        let ciphertext = data[NONCE_SIZE..].to_vec();

        Ok(Self { ciphertext, nonce })
    }
}

/// Encryption error types
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Invalid key length")]
    InvalidKey,

    #[error("Invalid nonce length")]
    InvalidNonce,

    #[error("Ciphertext too short (missing auth tag?)")]
    CiphertextTooShort,

    #[error("Authentication failed (ciphertext corrupted or wrong key)")]
    AuthenticationFailed,

    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,
}

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Encrypt data using AES-256-GCM with a random nonce
///
/// # Arguments
/// * `plaintext` - Data to encrypt
/// * `key` - 32-byte AES-256 key
/// * `nonce` - 12-byte nonce/IV
///
/// # Returns
/// * `EncryptedBlob` containing ciphertext and nonce
pub fn encrypt_data(
    plaintext: &[u8],
    key: &[u8; AES_KEY_SIZE],
    nonce: &NonceBytes,
) -> EncryptionResult<EncryptedBlob> {
    if key.len() != AES_KEY_SIZE {
        return Err(EncryptionError::InvalidKey);
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::InvalidKey)?;

    // Create nonce as Nonce<Aes256Gcm> (GenericArray<u8, U12>)
    let nonce_arr = Nonce::from_slice(nonce.as_bytes());

    let ciphertext = cipher
        .encrypt(nonce_arr, plaintext)
        .map_err(|_| EncryptionError::EncryptionFailed)?;

    Ok(EncryptedBlob {
        ciphertext,
        nonce: nonce.0.to_vec(),
    })
}

/// Decrypt data using AES-256-GCM
///
/// # Arguments
/// * `ciphertext` - Encrypted data (includes auth tag)
/// * `key` - 32-byte AES-256 key
/// * `nonce` - 12-byte nonce/IV used during encryption
///
/// # Returns
/// * Decrypted plaintext
pub fn decrypt_data(
    ciphertext: &[u8],
    key: &[u8; AES_KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
) -> EncryptionResult<Vec<u8>> {
    if key.len() != AES_KEY_SIZE {
        return Err(EncryptionError::InvalidKey);
    }

    if ciphertext.len() < TAG_SIZE {
        return Err(EncryptionError::CiphertextTooShort);
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::InvalidKey)?;

    let nonce_arr = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce_arr, ciphertext)
        .map_err(|_| EncryptionError::AuthenticationFailed)
}

/// AES-256-GCM encrypt (internal use with raw bytes)
pub fn aes_encrypt(
    plaintext: &[u8],
    key: &[u8; AES_KEY_SIZE],
    nonce: &NonceBytes,
) -> EncryptionResult<Vec<u8>> {
    let blob = encrypt_data(plaintext, key, nonce)?;
    Ok(blob.to_bytes())
}

/// AES-256-GCM decrypt (internal use with raw bytes)
pub fn aes_decrypt(encrypted_data: &[u8], key: &[u8; AES_KEY_SIZE]) -> EncryptionResult<Vec<u8>> {
    let blob = EncryptedBlob::from_bytes(encrypted_data)?;
    decrypt_data(
        &blob.ciphertext,
        key,
        &blob
            .nonce
            .try_into()
            .map_err(|_| EncryptionError::InvalidNonce)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"Hello, Xavier E2E Encryption!";

        let blob = encrypt_data(plaintext, &key, &nonce).expect("test assertion");
        let decrypted = decrypt_data(&blob.ciphertext, &key, nonce.as_bytes()).expect("test assertion");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypted_blob_serialization() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"Test data for serialization";

        let blob = encrypt_data(plaintext, &key, &nonce).expect("test assertion");
        let serialized = blob.to_bytes();
        let deserialized = EncryptedBlob::from_bytes(&serialized).expect("test assertion");

        let decrypted = decrypt_data(
            &deserialized.ciphertext,
            &key,
            &deserialized.nonce.try_into().expect("test assertion"),
        )
        .expect("test assertion");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let key2 = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"Secret message";

        let blob = encrypt_data(plaintext, &key1, &nonce).expect("test assertion");

        // Decrypt with wrong key should fail (authentication error)
        let result = decrypt_data(&blob.ciphertext, &key2, nonce.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"Original message";

        let blob = encrypt_data(plaintext, &key, &nonce).expect("test assertion");
        let mut tampered = blob.ciphertext.clone();
        // Flip a bit in the middle
        if tampered.len() > 20 {
            tampered[20] ^= 0xFF;
        }

        let result = decrypt_data(&tampered, &key, nonce.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_aes_encrypt_decrypt() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"AES encrypt/decrypt test";

        let encrypted = aes_encrypt(plaintext, &key, &nonce).expect("test assertion");
        let decrypted = aes_decrypt(&encrypted, &key).expect("test assertion");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_nonce_from_dek() {
        let dek = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };

        let nonce = NonceBytes::from_dek(&dek);
        assert_eq!(nonce.0, dek[..12]);

        // Same DEK always produces same nonce
        let nonce2 = NonceBytes::from_dek(&dek);
        assert_eq!(nonce.0, nonce2.0);

        // Different DEK produces different nonce
        let dek2 = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce3 = NonceBytes::from_dek(&dek2);
        assert_ne!(nonce.0, nonce3.0);
    }

    #[test]
    fn test_empty_plaintext() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        let plaintext = b"";

        let blob = encrypt_data(plaintext, &key, &nonce).expect("test assertion");
        let decrypted = decrypt_data(&blob.ciphertext, &key, nonce.as_bytes()).expect("test assertion");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_large_plaintext() {
        let key = {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            k
        };
        let nonce = NonceBytes::generate();
        // 1MB of data
        let plaintext = vec![0xAB; 1024 * 1024];

        let blob = encrypt_data(&plaintext, &key, &nonce).expect("test assertion");
        let decrypted = decrypt_data(&blob.ciphertext, &key, nonce.as_bytes()).expect("test assertion");

        assert_eq!(plaintext, decrypted);
    }
}
