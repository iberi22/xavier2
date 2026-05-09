# XAVIER ENCRYPTED PROTOCOL — SPEC v1.0

## Summary

All inter-node communication in Xavier is encrypted using AES-256-GCM over TLS 1.3. Even if traffic is intercepted, data is unreadable without the session key.

---

## Encryption Architecture

### Threat Model
```
Attacker intercepts network traffic → Sees ONLY encrypted data
Malicious agent sends injection → Blocked by Anticipator
Unauthorized node tries access → Blocked by node authentication
```

### Protocol Flow

```
Node A                    Xavier Server              Node B
   │                          │                        │
   │──── /auth/register ─────►│                        │
   │◄─── node_token + master_key (secure channel) ───│
   │                          │                        │
   │──── /auth/handshake ─────►│                        │
   │◄─── session_key ─────────│                        │
   │                          │                        │
   │ Encrypt(request)         │                        │
   │──── AES-256-GCM ─────────►│                        │
   │     (payload + IV + node_id)                       │
   │                          │                        │
   │                          │◄─── AES-256-GCM ─────│
   │                          │      (same session)   │
```

---

## Key Exchange Protocol

### 1. Node Registration

```http
POST /auth/register
Content-Type: application/json

{
  "node_id": "agent-ventas-001",
  "node_type": "openclaw_agent",
  "public_key": "base64_encoded_public_key"
}
```

Response:
```json
{
  "node_token": "ntk_xxxxx",
  "master_key_hint": "identifier_for_key",
  "expires_at": 1701993600
}
```

### 2. Session Key Derivation

```http
POST /auth/handshake
X-Xavier-Token: ntk_xxxxx
Content-Type: application/json

{
  "node_id": "agent-ventas-001",
  "challenge": "random_server_challenge"
}
```

Server derives session_key:
```rust
session_key = HKDF-SHA256(master_key, node_id || timestamp)
```

Response:
```json
{
  "session_key": "base64_encoded_session_key",
  "iv": "12_byte_nonce",
  "expires_at": 1701907200
}
```

---

## Encrypted Request Format

```json
{
  "version": 1,
  "encrypted": true,
  "node_id": "agent-ventas-001",
  "timestamp": 1701907200,
  "nonce": "12_bytes_base64",
  "payload": "base64_aes256gcm_output",
  "auth_tag": "base64_gcm_auth_tag"
}
```

---

## Cryptographic Algorithms

| Component | Algorithm | Notes |
|-----------|-----------|-------|
| **Symmetric Encryption** | AES-256-GCM | Authenticated encryption |
| **Key Derivation** | HKDF-SHA256 | Per-session keys |
| **IV/Nonce** | 12 bytes | Random per request |
| **Hash** | SHA-256 | Integrity verification |
| **TLS** | 1.3 minimum | Transport layer |

---

## Session Key Lifecycle

| Phase | Duration | Action |
|-------|----------|--------|
| Creation | — | Derived via HKDF |
| Active | 24 hours | Used for all requests |
| Expiry | — | Must re-handshake |
| Rotation | Daily | New session_key derived |

---

## Implementation (Rust)

### Dependencies (Cargo.toml)
```toml
aes-gcm = "0.10"
hkdf = "0.12"
sha2 = "0.10"
rand = "0.8"
base64 = "0.21"
```

### EncryptedChannel Struct

```rust
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use hkdf::Hkdf;
use sha2::Sha256;

pub struct EncryptedChannel {
    session_key: [u8; 32],
}

impl EncryptedChannel {
    pub fn from_master(master_key: &[u8], node_id: &str) -> Self {
        let hk = Hkdf::<Sha256>::new(None, master_key);
        let mut session_key = [0u8; 32];
        hk.expand(format!("{}-{}", node_id, timestamp()).as_bytes(), &mut session_key)
            .expect("HKDF expand failed");
        Self { session_key }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .expect("Invalid key");
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .expect("Encryption failed");

        (ciphertext, nonce_bytes)
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Vec<u8> {
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .expect("Invalid key");
        let nonce = Nonce::from_slice(nonce);

        cipher.decrypt(nonce, ciphertext)
            .expect("Decryption failed")
    }
}
```

### API Middleware

```rust
async fn encrypted_handler(req: Request) -> Result<Response, Error> {
    // Extract encrypted payload
    let encrypted: EncryptedRequest = req.json()?;

    // Verify node exists and session is valid
    let node = validate_node(&encrypted.node_id)?;
    if node.session_expired() {
        return Err(Error::SessionExpired);
    }

    // Decrypt payload
    let channel = EncryptedChannel::from_session(&node.session_key);
    let plaintext = channel.decrypt(
        &base64::decode(&encrypted.payload)?,
        &base64::decode(&encrypted.nonce)?
    )?;

    // Process request
    let request: ApiRequest = serde_json::from_slice(&plaintext)?;
    process(request).await
}
```

---

## Security Properties

| Property | Guarantee |
|----------|-----------|
| **Confidentiality** | AES-256-GCM encryption, keys never in transit |
| **Integrity** | GCM auth tag prevents tampering |
| **Authentication** | Node tokens + session keys |
| **Forward Secrecy** | Session keys rotated daily |
| **Replay Prevention** | Timestamp + nonce validation |

---

## Backwards Compatibility

For local-only deployments (Free tier), encryption is optional:

```http
# Unencrypted (local only)
POST /memory/add

# Encrypted (cloud tier)
POST /secure/add
```

---

## Testing

```bash
# Test encryption roundtrip
cargo test encrypted_channel

# Test key derivation
cargo test key_derivation

# Test decryption failures
cargo test decryption_failures
```

---

## Status

| Component | Status |
|-----------|--------|
| Protocol design | ✅ Complete |
| Rust implementation | TODO |
| Integration with API | TODO |
| Key rotation | TODO |
| Performance testing | TODO |
