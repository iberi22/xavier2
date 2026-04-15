//! Crypto Module - E2E Encryption for Xavier2 Cloud Tier
//!
//! This module provides end-to-end encryption for the cloud storage tier.
//! The server NEVER sees plaintext or keys - all encryption/decryption happens client-side.
//!
//! # Architecture
//!
//! ```text
//! User Data → Encrypted on Client → Stored Encrypted in Cloud → Never Decrypted by Server
//! ```
//!
//! # Key Hierarchy
//!
//! - **KEK (Key Encryption Key)**: Derived from user password via Argon2id
//! - **DEK (Data Encryption Key)**: Per-document random key, encrypted with KEK
//!
//! # Encryption Flow
//!
//! ```text
//! User Password → Argon2id → KEK
//! DEK = GenerateRandomKey(32 bytes)
//! Encrypted_DEK = AES-256-GCM(DEK, KEK, iv_kek)
//! Encrypted_Data = AES-256-GCM(plaintext, DEK, iv_data)
//! Store: Encrypted_DEK + Encrypted_Data + iv_kek + iv_data + salt
//! ```

pub mod encryption;
pub mod keys;

pub use encryption::{decrypt_data, encrypt_data, EncryptedBlob};
pub use keys::{derive_kek_from_password, generate_dek, KeyManager};

/// Size of DEK (Data Encryption Key) in bytes
pub const DEK_SIZE: usize = 32;

/// Size of salt for Argon2 in bytes
pub const SALT_SIZE: usize = 16;

/// Size of nonce/IV for AES-256-GCM in bytes
pub const NONCE_SIZE: usize = 12;
