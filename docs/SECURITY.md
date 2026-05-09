# XAVIER SECURITY ARCHITECTURE

## Overview

Two-layer security system:
1. **Encryption** — End-to-end encryption for all inter-node communication
2. **Anticipator** — Prompt injection detection for multi-agent messages

---

## Layer 1: End-to-End Encryption

### Threat Model
- Man-in-the-middle attacks on HTTP/WebSocket
- Network traffic interception
- Unauthorized node access

### Solution: AES-256-GCM + TLS

```
┌─────────────┐     TLS (WSS)      ┌─────────────┐
│   Node A    │ ────────────────► │   Node B    │
│  (Agent)    │                   │  (Xavier)   │
│             │ ◄── E2E Encrypted │             │
└─────────────┘                   └─────────────┘
```

### Encryption Flow

```
1. Node registers → receives node_token + encryption_key (secure channel)
2. All API requests:
   a. Payload encrypted with AES-256-GCM using session_key
   b. Encrypted payload + IV sent via TLS
   c. Server decrypts with session_key
3. Responses encrypted same way (session_key is symmetric)
```

### Key Management

| Key Type | Purpose | Storage |
|----------|---------|---------|
| **Master Key** | Derive session keys | Environment variable (never in code) |
| **Session Key** | Per-request encryption | Ephemeral, rotated daily |
| **Node Token** | Authentication | Node local storage |

### Algorithm
- **Encryption**: AES-256-GCM (authenticated encryption)
- **Key Derivation**: HKDF-SHA256
- **IV**: 12 bytes, randomly generated per request
- **TLS**: 1.3 minimum, WSS for WebSocket

### Implementation (Rust)

```rust
// src/security/encryption.rs

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
    /// Derive session key from master_key + node_id
    pub fn new_session(master_key: &[u8], node_id: &str) -> Self {
        let hk = Hkdf::<Sha256>::new(None, master_key);
        let mut session_key = [0u8; 32];
        hk.expand(node_id.as_bytes(), &mut session_key)
            .expect("HKDF expand failed");
        Self { session_key }
    }

    /// Encrypt payload with AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .expect("Invalid key length");
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .expect("Encryption failed");

        (ciphertext, nonce_bytes)
    }

    /// Decrypt payload with AES-256-GCM
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Vec<u8> {
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .expect("Invalid key length");
        let nonce = Nonce::from_slice(nonce);

        cipher
            .decrypt(nonce, ciphertext)
            .expect("Decryption failed")
    }
}
```

### API Changes

#### Encrypted Request
```json
{
  "encrypted": true,
  "iv": "base64_12_bytes",
  "payload": "base64_aes256gcm_ciphertext",
  "node_id": "node_abc123",
  "timestamp": 1701907200
}
```

#### New Endpoints
```
POST /auth/register     → Register node, get node_token
POST /auth/handshake   → Exchange session key
POST /secure/search     → Encrypted search
POST /secure/add        → Encrypted add memory
```

---

## Layer 2: Anticipator Integration

### What is Anticipator?
Runtime security for multi-agent AI systems. Detects prompt injection, credential leakage, encoding attacks, homoglyph spoofing, and path traversal — **before they become incidents**.

**10 Detection Layers:**

| Layer | Method | Catches |
|-------|-------|---------|
| Phrase Detection | Aho-Corasick | Injection commands, role switches, jailbreak |
| Encoding Detection | Base64/Hex/URL decode | Obfuscated payloads |
| Entropy Detection | Shannon + regex | API keys, JWTs, tokens, secrets |
| Heuristic Detection | Pattern matching | Spacing tricks, ALL CAPS, role-switch |
| Canary Detection | Token injection | Context leakage, watermark exfiltration |
| Homoglyph Detection | Unicode normalization | Cyrillic spoofing, zero-width chars |
| Path Traversal | Pattern + URL decode | ../, /etc/passwd, .aws/credentials |
| Tool Alias Detection | Name fuzzing | Spoofed tool calls |
| Threat Categories | Multi-class classifier | Authority escalation, social engineering |
| Config Drift | Config snapshot diffing | Runtime tampering |

### Integration with Xavier

```
User Input
    │
    ▼
┌──────────────────┐
│  Anticipator     │ ◄── Scan every inter-agent message
│  (10 layers)     │
└────────┬─────────┘
         │ Clean → Forward to Xavier
         │ Threat → Log + Alert + (optional block)
         ▼
┌──────────────────┐
│   Xavier API     │
│   /memory/*     │
└──────────────────┘
```

### Implementation (Rust/WASM)

Since Anticipator is Python/LangGraph based, we have options:

1. **WASM Compilation** — Compile Anticipator's core logic to WASM
2. **Sidecar Service** — Python microservice that wraps Anticipator
3. **API Proxy** — Rust proxy that calls Anticipator REST API
4. **Re-implementation** — Port core layers to Rust (recommended for production)

### Recommended: Rust Port of Core Layers

```rust
// src/security/anticipator_core.rs

pub enum ThreatLevel {
    Clean,
    Warning,
    Critical,
}

pub struct ScanResult {
    pub level: ThreatLevel,
    pub layers_triggered: Vec<String>,
    pub details: String,
}

pub fn scan_message(message: &str) -> ScanResult {
    let mut triggered = Vec::new();

    // Layer 1: Phrase Detection (Aho-Corasick)
    if detect_injection_phrases(message) {
        triggered.push("phrase_injection".to_string());
    }

    // Layer 2: Encoding Detection
    if contains_encoded_payload(message) {
        triggered.push("encoding".to_string());
    }

    // Layer 3: Entropy Detection (secrets)
    if contains_high_entropy(message) {
        triggered.push("entropy".to_string());
    }

    // Layer 4: Homoglyph Detection
    if contains_homoglyphs(message) {
        triggered.push("homoglyph".to_string());
    }

    // Layer 5: Path Traversal
    if contains_path_traversal(message) {
        triggered.push("path_traversal".to_string());
    }

    match triggered.len() {
        0 => ScanResult { level: ThreatLevel::Clean, layers_triggered: vec![], details: String::new() },
        1..=2 => ScanResult { level: ThreatLevel::Warning, layers_triggered: triggered, details: format!("{:?}", triggered) },
        _ => ScanResult { level: ThreatLevel::Critical, layers_triggered: triggered, details: format!("{:?}", triggered) },
    }
}
```

### Xavier API Changes

```rust
// Before processing any /memory/* request
fn scan_and_process(req: Request) -> Response {
    let scan = anticipator_core::scan_message(&req.body);

    match scan.level {
        ThreatLevel::Clean => process_normal(req),
        ThreatLevel::Warning => {
            log::warn!("Anticipator warning: {:?}", scan);
            process_normal(req) // Still process, but log
        }
        ThreatLevel::Critical => {
            log::error!("Anticipator CRITICAL: {:?}", scan);
            // Option: block request
            return Response::error("Threat detected", 403);
        }
    }
}
```

### OpenClaw Integration Note

According to Anticipator's README:
> **Openclaw** — 🔜 Coming soon

We can be the first to implement this natively.

---

## Security Checklist

| Item | Status | Notes |
|------|--------|-------|
| TLS 1.3 for transport | TODO | Required for all external endpoints |
| AES-256-GCM E2E | TODO | Session key per node |
| Anticipator Core | TODO | Port to Rust or WASM |
| Node authentication | TODO | node_token + session_key |
| Secret rotation | TODO | Daily key rotation |
| Audit logging | TODO | All security events logged |
| Rate limiting | TODO | Prevent DoS |
| Input validation | TODO | Sanitize all inputs |

---

## Implementation Roadmap

### Phase 1: Basic Security (Week 1)
- [ ] TLS 1.3 enforced on all endpoints
- [ ] API token authentication
- [ ] Basic rate limiting

### Phase 2: E2E Encryption (Week 2)
- [ ] AES-256-GCM implementation
- [ ] Key management system
- [ ] Encrypted channel for agents

### Phase 3: Anticipator Integration (Week 3)
- [ ] Port core detection layers to Rust
- [ ] WASM compilation for cross-platform
- [ ] Integration with Xavier API

### Phase 4: Hardening (Week 4)
- [ ] Penetration testing
- [ ] Security audit
- [ ] Compliance documentation
