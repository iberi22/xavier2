# Xavier Dependency Reduction Strategy

> **Purpose:** Analyze Xavier's Rust dependency tree and propose which crates can be replaced with custom implementations to reduce external dependencies, keep core logic proprietary, and adopt modern Rust patterns.
>
> **Last Updated:** 2026-04-06
> **Xavier Version:** 0.4.1
> **License:** Apache-2.0 (core), BSD-like for SurrealDB fork

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Dependency Inventory](#dependency-inventory)
3. [KEEP: Essential Dependencies](#keep-essential-dependencies)
4. [REPLACE: Custom Implementations](#replace-custom-implementations)
5. [MINIMIZE: Reduce Scope](#minimize-reduce-scope)
6. [License Implications](#license-implications)
7. [Modern Rust Patterns](#modern-rust-patterns)
8. [Implementation Order](#implementation-order)
9. [Critical Analysis: SurrealDB](#critical-analysis-surrealdb)
10. [Quick Reference Tables](#quick-reference-tables)

---

## Executive Summary

Xavier currently has **35 direct dependencies** across the main crate and `code-graph` workspace member. After analysis, we categorize them as:

| Category | Count | Risk | Proprietary Value |
|----------|-------|------|-------------------|
| **KEEP** (battle-tested) | 11 | Low | Low |
| **REPLACE** (custom impl) | 13 | Medium | High |
| **MINIMIZE** (reduce scope) | 8 | Low | Medium |
| **REMOVE** (unused) | 3 | — | — |

**Key Finding:** The core intellectual property — memory graph, belief system, embedding pipeline, and agent runtime — is **not** protected by any dependency license. It lives in application code. The dependency strategy should focus on replacing generic utilities while keeping battle-tested async infrastructure.

---

## Dependency Inventory

### Main Crate (`xavier/`)

```
tokio              1.50.0   [full]              async runtime
axum               0.8                        HTTP server
tower              0.5                        middleware utilities
tower-http         0.6       [cors, compression-full]  ← LIKELY UNUSED
serde              1.0.228   [derive]           serialization
serde_json         1.0.149                     JSON parsing
surrealdb          3.0.5    [protocol-ws]       database
rusqlite           0.32.0   [bundled]          SQLite
anyhow             1.0.102                      error handling
thiserror          2.0.18                       error types
async-trait        0.1                         async trait support
uuid               1.8       [v4]              UUID generation
chrono             0.4        [serde]           datetime
walkdir            2                            directory traversal
regex              1.10                        regex engine
sha2               0.10                        SHA256 hashing
hex                0.4                          hex encoding
flate2             1.0                          gzip compression
clap               4          [derive]          CLI args
ratatui            0.29                         TUI library
crossterm          0.28                         terminal I/O
tracing            0.1                          observability
tracing-subscriber 0.3       [env-filter]      log subscriber
reqwest            0.12      [json]             HTTP client
tokio-stream       0.1                          async streams
parking_lot        0.12                         sync primitives
cron               0.15                         cron parsing
surrealdb-types    3.0.5                        DB types
```

### `code-graph/` Workspace Member

```
tokio              1         [full]            async runtime
clap               4          [derive, env]    CLI
tree-sitter        0.23                        parser infrastructure
tree-sitter-rust   0.23                        Rust parser
tree-sitter-python 0.25                        Python parser
tree-sitter-typescript 0.23                    TS/JS parser
tree-sitter-go     0.25                        Go parser
tree-sitter-java   0.23                        Java parser
walkdir            2                            directory traversal
serde              1           [derive]         serialization
serde_json         1                            JSON
thiserror          2                            error types
anyhow             1                            error handling
tracing            0.1                          observability
tracing-subscriber 0.3        [env-filter]      log subscriber
axum               0.8                         HTTP server
tower              0.5        [util]            middleware utilities
tower-http         0.6        [cors]            ← LIKELY UNUSED
hyper              1                            HTTP primitives
hyper-util         0.1                          HTTP utilities
parking_lot        0.12                         sync primitives
rusqlite           0.32.0   [bundled]          SQLite
```

---

## KEEP: Essential Dependencies

These are battle-tested, industry-standard crates where replacing would cost more than it's worth. The async runtime, HTTP server, and serialization foundations are **not** where proprietary value lives.

### 1. `tokio` — Async Runtime

**Why Keep:**
- The entire async ecosystem depends on it. Replacing with `async-std` or smol would buy us nothing.
- `tokio` is as fast as hand-written C for I/O scheduling.
- All other async crates (axum, surrealdb, reqwest) assume tokio.

**What We Actually Use:**
```toml
tokio = { version = "1.50.0", features = ["full"] }
```

**Reality Check:** We compile with `features = ["full"]` which is heavy. We should reduce to specific features (see MINIMIZE section).

---

### 2. `axum` — HTTP Server

**Why Keep:**
- Type-safe routing, middleware, and extractors are excellent.
- Built on `hyper` with excellent performance.
- Ecosystem integration (tower, tower-http) is unmatched.
- Replacing with raw `hyper` would be ~5,000 lines of boilerplate.

**What We Actually Use:**
- `Router`, `routing::{get, post, delete}`
- `middleware::from_fn_with_state`, `middleware::from_fn`
- `extract::{Query, State, Path, Json, Extension}`
- `response::IntoResponse`
- `http::Request/Response`, `StatusCode`

**Verdict:** Keep. The HTTP layer is commodity infrastructure.

---

### 3. `serde` + `serde_json` — Serialization

**Why Keep:**
- The de-facto standard for Rust serialization. Every crate integrates with it.
- `#[derive(Serialize, Deserialize)]` is ergonomic and zero-cost.
- Custom implementations would need to replicate the derive macro ecosystem.
- Used for: JSON API, config files, memory records, checkpoint files.

**What We Actually Use:**
- `Serialize`, `Deserialize`, `DeserializeOwned` derive macros
- `serde_json::Value`, `serde_json::json!()` macro
- `serde_json::to_string`, `from_str`, `to_string_pretty`

**Verdict:** Keep. Serialization is commodity infrastructure.

---

### 4. `rusqlite` — SQLite (code-graph only)

**Why Keep:**
- Embedded, zero-configuration, persistent key-value store for code-graph indices.
- `bundled` feature compiles SQLite from source — no system dependency.
- Tree-sitter + SQLite is a proven combination for code search (used by GitHub).
- We use it for: symbol index, graph storage, query cache.

**Verdict:** Keep for `code-graph`. This is an implementation detail of a supporting subsystem.

---

### 5. `tree-sitter-*` — Code Parsers (code-graph only)

**Why Keep:**
- Deterministic, correct syntax tree extraction for multiple languages.
- Replacing with custom parsers would be years of work.
- MIT licensed — no license contamination.

**What We Actually Use:**
- `tree-sitter` core
- `tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-typescript`, `tree-sitter-go`, `tree-sitter-java`
- Language detection, AST traversal, symbol extraction

**Verdict:** Keep in `code-graph`. This is infrastructure for the code-understanding subsystem.

---

### 6. `anyhow` — Error Handling

**Why Keep:**
- `anyhow::Result<T>` with `?` operator is the most ergonomic error handling in Rust.
- Contextual errors (`context()`) are invaluable for debugging.
- We use it in nearly every module. Replacing with `thiserror` everywhere would be massive churn.

**What We Actually Use:**
- `anyhow::Result`, `anyhow::Context`, `anyhow::anyhow!`, `anyhow::bail!`
- No `anyhow::Error` boxing in hot paths

**Recommendation:** Keep `anyhow` for application-level error handling. Use `thiserror` for library/interface boundaries.

---

### 7. `reqwest` — HTTP Client (for Embeddings)

**Why Keep:**
- The embedding client needs a proper HTTP/JSON client.
- `Client::builder().timeout().build()` is production-ready.
- Replacing with raw `hyper` + `http` would be ~200 lines of boilerplate.

**What We Actually Use:**
- `Client`, `Client::builder().timeout().build()`
- `.post(url).json(&body).send().await`
- `.json::<T>().await` response parsing

**Verdict:** Keep. Consider `ureq` as a lighter alternative if we don't need async.

---

### 8. `clap` — CLI Argument Parsing

**Why Keep:**
- `#[derive(Parser)]` is the gold standard for CLI parsing in Rust.
- Used only in `main.rs` and `code-graph/src/main.rs`.
- Replacing would buy minimal reduction in dependency count.

**Verdict:** Keep.

---

### 9. `ratatui` + `crossterm` — TUI

**Why Keep:**
- Used only for the TUI dashboard binary (`xavier-tui`).
- TUI is a terminal interface — not proprietary.
- Replacing would require reimplementing layout, rendering, and event handling.

**Verdict:** Keep for the TUI binary only. Not in the core library.

---

### 10. `tracing` + `tracing-subscriber` — Observability

**Why Keep:**
- Structured logging is essential for production debugging.
- `tracing` is the ecosystem standard (tokio, axum, tower all use it).
- Replacing with `log` crate + custom impl would be equivalent work.

**What We Actually Use:**
- `tracing::info!`, `tracing::warn!`, `tracing::debug!`
- `tracing_subscriber::registry().with(EnvFilter::new()).with(fmt::layer())`

**Verdict:** Keep. Observability is infrastructure.

---

### 11. `parking_lot` — Sync Primitives

**Why Keep:**
- `RwLock`, `Mutex` are faster than std's `sync::Mutex` (no poisoning semantics, better scheduling).
- Used for shared state in `AppState`, `WorkspaceRegistry`, etc.
- Small, well-tested crate.

**Verdict:** Keep.

---

## REPLACE: Custom Implementations

These are **high-value targets** for custom implementations. They are generic utilities where our specific use cases are narrow enough to implement ourselves.

### Priority 1: `hex` — Hexadecimal Encoding

**Current Usage:** Only `hex::encode(digest)` where digest is a `Sha256` output.

**Proposed Replacement:**

```rust
/// Encode bytes as lowercase hexadecimal string.
/// Replaces: `hex::encode(sha256(data))`
pub fn hex_encode(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for &byte in data {
        out.push(HEX_CHARS[(byte >> 4) as usize] as char);
        out.push(HEX_CHARS[(byte & 0xf) as usize] as char);
    }
    out
}
```

**Effort:** 30 minutes. One function.

**Files to Change:**
- `src/server/http.rs` (query fingerprint)
- `src/server/mcp_server.rs` (token hashing)
- `src/agents/runtime.rs` (belief hashing)
- `src/sync/chunks.rs` (chunk hashing)

**Estimated Time:** 1-2 hours (including tests).

---

### Priority 2: `uuid` — UUID v4 Generation

**Current Usage:** Only `uuid::Uuid::new_v4()` for generating session IDs, thread IDs, request IDs.

**Proposed Replacement:**

```rust
use std::sync::atomic::{AtomicU64, Ordering};

// RFC 4122 v4 UUID — 122 bits of randomness encoded as:
// xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
fn new_v4_uuid() -> String {
    let mut bytes = [0u8; 16];

    // Fast 16-byte random fill
    // Use a CSPRNG seeded by OS entropy
    fill_random_bytes(&mut bytes);

    // Set version (4) and variant bits per RFC 4122
    bytes[6] = (bytes[6] & 0x0f) | 0x40;  // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80;  // variant 10

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
```

**Files to Change:** 6 files using `uuid::Uuid`:
- `src/coordination/message_bus.rs`
- `src/coordination/mod.rs`
- `src/memory/session_store.rs`
- `src/server/mcp_server.rs`
- `src/server/panel.rs`
- `src/tasks/models.rs`

**Estimated Time:** 1 day.

---

### Priority 3: `chrono` — DateTime

**Current Usage:**
- `chrono::{DateTime, Utc}` for timestamps
- `DateTime::parse_from_rfc3339()`
- `DateTime::with_timezone(&Utc)`
- `Utc::now()`
- `Datelike`, `NaiveDate`, `Duration` in `qmd_memory.rs` and `system3.rs`

**Proposed Replacement:**

```rust
/// Unix timestamp in milliseconds (compatible with JSON/serde)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcDateTime(i64);

impl UtcDateTime {
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        )
    }

    pub fn from_timestamp_millis(ms: i64) -> Self { Self(ms) }
    pub fn timestamp_millis(self) -> i64 { self.0 }
    pub fn as_secs(self) -> i64 { self.0 / 1000 }

    pub fn to_rfc3339(self) -> String {
        use std::time::UNIX_EPOCH;
        let secs = self.0 / 1000;
        let nanos = ((self.0 % 1000) * 1_000_000) as u32;
        let datetime = UNIX_EPOCH + std::time::Duration::new(secs, nanos);
        // Format as ISO 8601
        // ... (format the datetime)
        format!("{}Z", ISO_FORMAT_HERE)
    }

    pub fn parse_from_rfc3339(s: &str) -> Option<Self> {
        // Parse "2024-01-02T15:04:05.123456Z"
        // Returns None for malformed input
    }
}

impl serde::Serialize for UtcDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        // Serialize as i64 (unix timestamp millis)
        serializer.serialize_i64(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for UtcDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        let ms = i64::deserialize(deserializer)?;
        Ok(Self(ms))
    }
}
```

**Impact:** `chrono` is used across 15+ files. Full replacement is significant work but pays off in:
- Removing `features = ["serde"]` (no more chrono serde compat complexity)
- Owning our datetime handling
- Smaller compile times (chrono is a large crate)

**Effort:** 2-3 days for complete replacement across all files.

**Key Files (by usage volume):**
- `src/memory/surreal_store.rs` (extensive)
- `src/memory/schema.rs`
- `src/memory/manager.rs`
- `src/memory/session_store.rs`
- `src/agents/system3.rs`
- `src/checkpoint/state.rs`
- `src/scheduler/job.rs`

**Estimated Time:** 3-5 days.

---

### Priority 4: `sha2` — SHA256 Hashing

**Current Usage:**
- `sha2::{Digest, Sha256}::digest(data)` for hashing
- We only use SHA256. We don't need the full `Digest` trait machinery.

**Proposed Replacement:**

```rust
/// Pure-Rust SHA-256 implementation. ~300 lines.
/// Based on RFC 6234, public domain by FIPS 180-4.
/// Used for content-addressable storage keys and HMAC-style auth.

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    // ... (full K array, 64 u32 constants)
];

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = [
        0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: append padding
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 { msg.push(0x00); }
    let len_bits = (data.len() as u64) << 3;
    msg.extend_from_slice(&len_bits.to_be_bytes());

    // Process 512-bit chunks
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        let mut a = h[0]; let mut b = h[1]; let mut c = h[2]; let mut d = h[3];
        let mut e = h[4]; let mut f = h[5]; let mut g = h[6]; let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(temp1);
            d = c; c = b; b = a; a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for i in 0..8 { result[i*4..(i+1)*4].copy_from_slice(&h[i].to_be_bytes()); }
    result
}
```

**Effort:** 2-3 days to implement, test against FIPS 180-4 test vectors, and replace all usages.

**Files to Change:** 7 files using SHA256.

**Estimated Time:** 3-4 days.

---

### Priority 5: `regex` — Regex Engine

**Current Usage:** Only in `prompt_guard.rs` — prompt injection detection. Pre-compiled patterns at static initialization.

**Proposed Replacement:**

The prompt guard uses a **fixed set of pre-compiled regexes**. This is the key insight — we don't need a general-purpose regex engine. We can use:

1. **Aho-Corasick** (MIT licensed, single-purpose) for multi-pattern matching
2. **Simple substring/prefix/suffix checks** for most patterns
3. **Our own mini-regex** for simple patterns like `(?i)ignore\s+instructions?`

```rust
// Approach: Use aho-corasick for efficient multi-pattern matching
// + custom pattern matchers for complex patterns

use aho_corasick::AhoCorasick;

pub struct PromptInjectionDetector {
    // Aho-Corasick for exact multi-pattern matching
    ac_direct: AhoCorasick,
    ac_indirect: AhoCorasick,
    ac_leaking: AhoCorasick,
}

impl PromptInjectionDetector {
    pub fn new() -> Self {
        // Build patterns at construction time
        let direct_patterns = [
            "ignore all previous instructions",
            "ignore prior instructions",
            "forget everything",
            "you are now not an AI",
            "new system instructions",
            "override your safety",
            "disregard all rules",
            // ... (all current patterns)
        ];

        Self {
            ac_direct: AhoCorasick::new(&direct_patterns).unwrap(),
            // ...
        }
    }

    pub fn detect(&self, text: &str) -> DetectionResult {
        // Fast Aho-Corasick scan first
        // For complex patterns, fall back to custom checkers
    }
}
```

**Why not general regex:** The current patterns are mostly:
- Fixed strings (`"ignore instructions"`)
- Simple prefixes/suffixes (`"(?i)ignore\s+"`)
- OR patterns (`"leaking|extract|show.*prompt"`)

None require backreferences, lookahead, or complex regex features.

**Effort:** 2-3 days to rewrite prompt_guard.rs.

**Estimated Time:** 2-3 days.

---

### Priority 6: `walkdir` — Directory Traversal

**Current Usage:** Only in `bridge.rs` for importing from directory trees.

**Proposed Replacement:** We already have the pattern in `file_indexer.rs` — it uses a manual stack-based traversal. We can extract this into a utility:

```rust
/// Simple recursive directory iterator (no external crate needed).
/// Yields all file paths under `root` recursively.
pub fn walkdir(root: &Path) -> impl Iterator<Item = std::io::Result<PathBuf>> {
    WalkDirIter::new(root)
}

struct WalkDirIter {
    stack: Vec<PathBuf>,
}

impl WalkDirIter {
    fn new(root: &Path) -> Self {
        Self { stack: vec![root.to_path_buf()] }
    }
}

impl Iterator for WalkDirIter {
    type Item = std::io::Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(path) = self.stack.pop() {
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            self.stack.push(p);
                        } else {
                            return Some(Ok(p));
                        }
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
        None
    }
}
```

Or simply use `std::fs::read_dir` directly in `bridge.rs` with a recursive function — the bridge is already async anyway.

**Effort:** 2-3 hours.

**Files to Change:** `src/memory/bridge.rs`

**Estimated Time:** Half a day.

---

### Priority 7: `async-trait` — Async Trait Support

**Current Usage:** Only in `surreal_store.rs` for the `MemoryStore` trait.

**Proposed Replacement:** Use RPITIT (Return Position Impl Trait In Trait) available in Rust 1.75+.

```rust
// BEFORE (requires async-trait crate):
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>>;
}

// AFTER (native async fn in traits — Rust 1.75+):
pub trait MemoryStore: Send + Sync {
    fn get(&self, id: &str) -> impl std::future::Future<Output = Result<Option<MemoryRecord>>> + Send;
}
```

**Effort:** Low — just remove `#[async_trait]` and adjust return types.

**Note:** If we replace SurrealDB anyway (see Critical Analysis), we may not need this trait at all.

**Estimated Time:** 1-2 hours.

---

### Priority 8: `flate2` — Gzip Compression

**Current Usage:** Only in `sync/chunks.rs` for reading/writing `.jsonl.gz` chunk files.

**Proposed Replacement:** Use `miniz_oxide` (pure Rust, MIT/Apache) or our own thin wrapper:

```rust
// Using miniz_oxide (pure Rust, well-maintained, MIT/Apache-2.0)
use miniz_oxide::{inflate::inflate_from_vec, deflate::deflate_to_vec_gzip};

pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    inflate_from_vec(data)
        .map_err(|e| anyhow!("gzip decompress failed: {:?}", e))
}

pub fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    deflate_to_vec_gzip(data, 6)  // compression level 6
}
```

**Why not pure implementation:** Gzip is complex (DEFLATE algorithm with zlib wrapper). Using `miniz_oxide` is cleaner and still pure Rust.

**Effort:** 1 day to evaluate and switch.

**Files to Change:** `src/sync/chunks.rs`

**Estimated Time:** 1 day.

---

### Priority 9: `cron` — Cron Parsing

**Current Usage:** `scheduler/mod.rs` — parsing cron expressions for scheduled tasks.

**Proposed Replacement:** Write a minimal cron parser (cron is a simple format):

```rust
#[derive(Clone)]
pub struct CronField {
    kind: CronFieldKind,
    min: u8,
    max: u8,
}

#[derive(Clone)]
enum CronFieldKind {
    Any,                    // *
    Single(u8),             // 5
    Range(u8, u8),          // 1-5
    Step(u8),               // */5 (every 5 units)
    List(Vec<u8>),          // 1,3,5
}

impl CronField {
    pub fn parse(s: &str, min: u8, max: u8) -> Result<Self> { /* ... */ }
    pub fn matches(&self, value: u8) -> bool { /* ... */ }
}

#[derive(Clone)]
pub struct CronSchedule {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

impl CronSchedule {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split_whitespace().collect();
        anyhow::ensure!(parts.len() == 5, "cron must have 5 fields");
        Ok(Self {
            minute: CronField::parse(parts[0], 0, 59)?,
            hour: CronField::parse(parts[1], 0, 23)?,
            day_of_month: CronField::parse(parts[2], 1, 31)?,
            month: CronField::parse(parts[3], 1, 12)?,
            day_of_week: CronField::parse(parts[4], 0, 6)?,
        })
    }

    pub fn matches(&self, dt: &ChronoDateTime) -> bool {
        self.minute.matches(dt.minute())
            && self.hour.matches(dt.hour())
            && self.day_of_month.matches(dt.day())
            && self.month.matches(dt.month())
            && self.day_of_week.matches(dt.weekday())
    }
}
```

**Effort:** 1-2 days for a complete, tested implementation.

**Files to Change:** `src/scheduler/mod.rs`

**Estimated Time:** 2 days.

---

### Priority 10: `surrealdb-types` — SurrealDB Value Types

**Current Usage:** Only `SurrealValue` derive on `MemoryRevision` and `MemoryRecord` in `surreal_store.rs`.

**Proposed Replacement:** Remove the derive and use plain serde:

```rust
// Instead of:
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct MemoryRecord { ... }

// Use:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord { ... }
```

`SurrealValue` is only needed for SurrealDB's own query language integration. We're not using that — we use SurrealDB as a dumb document store. Plain serde is sufficient.

**Effort:** 5 minutes. Remove the derive from 2 structs.

**Estimated Time:** 10 minutes.

---

### Priority 11: `tokio-stream` — Async Streams

**Current Usage:** Search codebase — appears to be declared but may not be actively used.

**Proposed Replacement:** Check if actually used. If not, remove. If yes, replace with `futures::stream::iter` or manual `Stream` impl.

**Effort:** 30 minutes to audit.

**Estimated Time:** 30 minutes.

---

## MINIMIZE: Reduce Scope

These dependencies are used, but with more features than needed.

### 1. `tokio` — Reduce Features

**Current:** `features = ["full"]` (compiles everything)
**Recommended:** Only what we use:

```toml
tokio = { version = "1.50.0", default-features = false, features = [
    "rt-multi-thread",    # Our async main uses multi-thread runtime
    "net",                # TCP/HTTP networking
    "fs",                 # File system operations
    "sync",               # RwLock, Mutex, etc.
    "time",               # sleep, interval
    "macros",             # #[tokio::main]
    "rt",                 # Runtime core
] }
```

**Impact:** Significantly faster compile times. `full` feature includes sync primitives, process management, and more that we don't use.

**Estimated Time:** 1 hour to audit all tokio usage, adjust features.

---

### 2. `tower-http` — Verify Usage

**Current:** `features = ["cors", "compression-full"]`
**Audit Result:** **UNUSED** in the codebase. We use axum's built-in CORS and our own middleware. `tower-http` is not imported anywhere in `src/`.

**Action:** Remove entirely. Both from `xavier/Cargo.toml` and `code-graph/Cargo.toml`.

**Impact:** Saves compilation of two middleware libraries.

**Estimated Time:** 15 minutes.

---

### 3. `tower` — Verify Full Feature Usage

**Current:** In `code-graph`, `features = ["util"]`
**Audit Result:** We use `tower::util::ServiceExt` in 3 places (`http.rs`, `mcp_server.rs`, `panel.rs`).

**Action:** Keep, but verify we don't need other tower features.

**Estimated Time:** 30 minutes.

---

### 4. `hyper` + `hyper-util` — Verify Usage

**Current:** In `code-graph`, `hyper = "1"` and `hyper-util = "0.1"`
**Audit Result:** These are dependencies of `axum`, not directly used.

**Action:** Remove from direct dependencies in `code-graph/Cargo.toml` (they'll still come in transitively through axum if needed).

**Estimated Time:** 30 minutes.

---

### 5. `anyhow` — Use `thiserror` at Boundaries

**Strategy:** Use `anyhow` for application code, `thiserror` for library/interface boundaries.

Currently, everything uses `anyhow`. We could define our public API error types with `thiserror`:

```rust
// In library/interface code:
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("storage error: {0}")]
    Storage(String),
}

// In application code:
type Result<T> = anyhow::Result<T>;
```

This is a refactoring task, not a replacement.

**Estimated Time:** 2-3 days across all modules.

---

### 6. `serde_json` — Reduce Feature Usage

**Current:** Default features (std, alloc)
**Actually Used:** `serde_json::Value`, `serde_json::json!()`, `to_string`, `from_str`, `from_reader`.

**Verdict:** No change needed. This is already minimal.

---

### 7. `tracing-subscriber` — Reduce Features

**Current:** `features = ["env-filter"]`
**Actually Used:** `EnvFilter`, `fmt::layer()`, `Registry`

**Action:** Keep `env-filter`. Already minimal.

---

### 8. `reqwest` — Reduce Features

**Current:** `features = ["json"]`
**Actually Used:** `.json()` method on response, JSON body building with serde.

**Action:** Keep `json` feature. Already minimal.

---

## LICENSE IMPLICATIONS

### Current Dependency Licenses

| Crate | License | Copyleft? | Risk |
|-------|---------|----------|------|
| tokio | MIT/Apache-2.0 | No | Low |
| axum | MIT/Apache-2.0 | No | Low |
| tower | MIT/Apache-2.0 | No | Low |
| tower-http | MIT/Apache-2.0 | No | Low |
| serde | MIT/Apache-2.0 | No | Low |
| serde_json | MIT/Apache-2.0 | No | Low |
| surrealdb | BSL → Apache-2.0* | No | Low |
| rusqlite | MIT | No | Low |
| anyhow | MIT/Apache-2.0 | No | Low |
| thiserror | MIT/Apache-2.0 | No | Low |
| async-trait | MIT/Apache-2.0 | No | Low |
| uuid | MIT/Apache-2.0 | No | Low |
| chrono | MIT/Apache-2.0 | No | Low |
| walkdir | BSD-2 | No | Low |
| regex | MIT/Apache-2.0 | No | Low |
| sha2 | MIT/Apache-2.0 | No | Low |
| hex | MIT/Apache-2.0 | No | Low |
| flate2 | MIT | No | Low |
| clap | MIT/Apache-2.0 | No | Low |
| ratatui | MIT | No | Low |
| crossterm | MIT | No | Low |
| tracing | MIT/Apache-2.0 | No | Low |
| tracing-subscriber | MIT/Apache-2.0 | No | Low |
| reqwest | MIT/Apache-2.0 | No | Low |
| tokio-stream | MIT/Apache-2.0 | No | Low |
| parking_lot | MIT/Apache-2.0 | No | Low |
| cron | MIT/Apache-2.0 | No | Low |
| surrealdb-types | Apache-2.0 | No | Low |
| tree-sitter-* | MIT | No | Low |
| hyper | MIT/Apache-2.0 | No | Low |
| hyper-util | MIT | No | Low |

### How BSD/SurrealDB-Style License Protects Our Code

**Key Distinction:** A dependency's license protects the dependency's code. It does **not** protect our code that uses the dependency.

```
Dependency License ←→ Our Code (using the dep)
MIT/Apache-2.0     → We can use, modify, distribute, sublicense
GPL               → If we link statically, we must open-source (too risky)
BSL               → Can use, but may have usage limits (commercial friendly)
```

**What SurrealDB's "BSD-like" license means:**
- SurrealDB will be Apache-2.0 (per their roadmap)
- Others CAN use SurrealDB without restriction
- **Our Xavier-specific logic** (memory graph, belief system, embeddings) is proprietary regardless of SurrealDB's license
- The license protects the **database engine**, not our **application logic**

### Proprietary Value Architecture

```
┌─────────────────────────────────────────────────────────┐
│  XAVIER APPLICATION CODE (Apache-2.0 or BSD, ours)      │
│  • Memory graph, belief system, embedding pipeline        │
│  • Agent runtime, task scheduler                         │
│  • API layer, UI, TUI                                   │
├─────────────────────────────────────────────────────────┤
│  SURREALDB (Apache-2.0, SurrealDB team + community)      │
│  • Query engine, storage engine, network protocol        │
│  • We USE SurrealDB, we don't OWN it                    │
├─────────────────────────────────────────────────────────┤
│  STANDARD DEPENDENCIES (all MIT/Apache-2.0)              │
│  tokio, axum, serde, etc. — no IP concerns              │
└─────────────────────────────────────────────────────────┘
```

**Bottom Line:** The IP is in our application code, not in our dependencies. Dependency replacement reduces supply chain risk and compile times, but doesn't materially change IP protection.

---

## MINIMIZE: Reduce Scope

These dependencies are used, but with more features than needed.

### 1. `tokio` — Reduce Features

**Current:** `features = ["full"]` (compiles everything)
**Recommended:** Only what we use:

```toml
tokio = { version = "1.50.0", default-features = false, features = [
    "rt-multi-thread",    # Our async main uses multi-thread runtime
    "net",                # TCP/HTTP networking
    "fs",                 # File system operations
    "sync",               # RwLock, Mutex, etc.
    "time",               # sleep, interval
    "macros",             # #[tokio::main]
    "rt",                 # Runtime core
] }
```

**Impact:** Significantly faster compile times. `full` feature includes sync primitives, process management, and more that we don't use.

**Estimated Time:** 1 hour to audit all tokio usage, adjust features.

---

### 2. `tower-http` — Verify Usage

**Current:** `features = ["cors", "compression-full"]`
**Audit Result:** **UNUSED** in the codebase. We use axum's built-in CORS and our own middleware. `tower-http` is not imported anywhere in `src/`.

**Action:** Remove entirely. Both from `xavier/Cargo.toml` and `code-graph/Cargo.toml`.

**Impact:** Saves compilation of two middleware libraries.

**Estimated Time:** 15 minutes.

---

### 3. `tower` — Verify Full Feature Usage

**Current:** In `code-graph`, `features = ["util"]`
**Audit Result:** We use `tower::util::ServiceExt` in 3 places (`http.rs`, `mcp_server.rs`, `panel.rs`).

**Action:** Keep, but verify we don't need other tower features.

**Estimated Time:** 30 minutes.

---

### 4. `hyper` + `hyper-util` — Verify Usage

**Current:** In `code-graph`, `hyper = "1"` and `hyper-util = "0.1"`
**Audit Result:** These are dependencies of `axum`, not directly used.

**Action:** Remove from direct dependencies in `code-graph/Cargo.toml`.

**Estimated Time:** 30 minutes.

---

### 5. `anyhow` — Use `thiserror` at Boundaries

**Strategy:** Use `anyhow` for application code, `thiserror` for library/interface boundaries.

Currently, everything uses `anyhow`. We could define our public API error types with `thiserror`:

```rust
// In library/interface code:
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("storage error: {0}")]
    Storage(String),
}

// In application code:
type Result<T> = anyhow::Result<T>;
```

This is a refactoring task, not a replacement.

**Estimated Time:** 2-3 days across all modules.

---

## Modern Rust Patterns

### 1. Async Traits: RPITIT vs `async-trait`

**Old Way (`async-trait` crate):**
```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>>;
}
```

**New Way (Rust 1.75+ RPITIT):**
```rust
pub trait MemoryStore: Send + Sync {
    fn get(&self, id: &str) -> impl std::future::Future<Output = Result<Option<MemoryRecord>>> + Send;
}
```

**When to use each:**
- Use RPITIT for new code (Rust 1.75+)
- Keep `async-trait` for now during transition
- The performance difference: RPITIT avoids a heap allocation from `Box<dyn Future>`

---

### 2. Type-Safe Error Handling with `?`

**Pattern:** Chain contextual errors with `context()`

```rust
async fn get_memory(ws: &str, id: &str) -> Result<Option<MemoryRecord>> {
    let store = get_store().await
        .context("failed to connect to memory store")?;

    store.get(ws, id).await
        .with_context(|| format!("failed to get memory record {} in workspace {}", id, ws))
}
```

---

### 3. Trait Objects Where Appropriate

**Pattern:** Use `dyn Trait` for runtime polymorphism

```rust
// Use Box<dyn SomeTrait> for plugin-like systems:
pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> impl std::future::Future<Output = Result<Vec<f32>>> + Send;
}
```

---

### 4. Using `std::future::Future` Directly

Rust's native async fns return impl Future, not a boxed type:

```rust
// GOOD: async fn returns impl Future (zero-cost)
async fn fetch_memory(id: &str) -> Result<Option<MemoryRecord>> {
    store.get(id).await
}

// BAD: boxing destroys zero-cost guarantee
fn fetch_memory(id: &str) -> Box<dyn Future<Output = Result<Option<MemoryRecord>>> + Send> {
    Box::new(async move { store.get(id).await })
}
```

---

### 5. `trait` Bounds: Where Clauses for Clarity

**Pattern:** Use `where` clauses for complex bounds

```rust
// Clear:
fn process<M, E>(memory: M) -> impl Future<Output = Result<()>>
where
    M: MemoryStore + Clone,
    E: Encoder;
```

---

## Implementation Order

### Phase 1: Quick Wins (Week 1)

| # | Task | Time | Risk | Value |
|---|------|------|------|-------|
| 1 | Remove `tower-http` (unused) | 15min | None | Compile speed |
| 2 | Remove `surrealdb-types` derive | 10min | None | Simplicity |
| 3 | Replace `hex` with custom impl | 2hr | Low | -1 dep |
| 4 | Replace `walkdir` in bridge.rs | 4hr | Low | -1 dep |
| 5 | Audit `tokio` features | 1hr | Low | Compile speed |
| 6 | Audit `tokio-stream` | 30min | None | -1 dep (maybe) |

**Phase 1 Total: ~1 day**

---

### Phase 2: Medium Effort (Week 2)

| # | Task | Time | Risk | Value |
|---|------|------|------|-------|
| 7 | Replace `uuid` with custom impl | 1 day | Medium | -1 dep |
| 8 | Replace `async-trait` with RPITIT | 2hr | Low | -1 dep |
| 9 | Replace `flate2` with `miniz_oxide` | 1 day | Low | +1 pure-Rust dep |
| 10 | Audit `hyper` + `hyper-util` removal | 30min | None | -2 deps |
| 11 | Replace `regex` in prompt_guard | 2-3 days | Medium | -1 dep, custom logic |

**Phase 2 Total: ~5 days**

---

### Phase 3: Major Refactors (Week 3-4)

| # | Task | Time | Risk | Value |
|---|------|------|------|-------|
| 12 | Replace `sha2` with custom impl | 3-4 days | Medium | -1 dep, own hash |
| 13 | Replace `chrono` with custom impl | 3-5 days | Medium-High | -1 large dep |
| 14 | Replace `cron` with custom impl | 2 days | Low | -1 dep |
| 15 | `thiserror` at API boundaries | 2-3 days | Low | Better errors |

**Phase 3 Total: ~10-14 days**

---

### Phase 4: Storage Layer (Ongoing)

See Critical Analysis: SurrealDB section below.

---

### Dependency Graph (What Depends on What)

```
async-trait ← MemoryStore trait ← surreal_store ← memory module
              ↓
         (replace with RPITIT, remove async-trait)

chrono ← scheduler/job, checkpoint/state, memory/*, agents/*
         ↓
    (widespread replacement, tackle last)

sha2 ← agents/runtime, sync/chunks, server/*, memory/surreal_store
       ↓
   (replace early, used in many places as hex(sha256(...)))

hex ← sha2 users (hex::encode(sha256(...)))
       ↓
   (replace first, simple)

walkdir ← memory/bridge.rs
          ↓
      (replace with std::fs::read_dir)

regex ← security/prompt_guard
        ↓
    (replace with aho-corasick + custom patterns)

uuid ← coordination, tasks, server/panel, memory/session_store
       ↓
   (replace with custom v4 impl)

flate2 ← sync/chunks
         ↓
     (replace with miniz_oxide)

cron ← scheduler/mod.rs
       ↓
   (custom parser)

surrealdb ← surreal_store (storage backend)
            ↓
        (see Critical Analysis)
```

---

## Critical Analysis: SurrealDB

### Should We Build Our Own Storage Layer?

**Short Answer:** Not yet. But architect for it.

SurrealDB provides:
1. Network protocol (WebSocket-based)
2. Query language (SurrealQL)
3. Storage engine (embedded or server mode)
4. ACID transactions
5. Schema validation

**What Xavier actually uses SurrealDB for:**
- Document storage (memory records as JSON)
- WebSocket connections for real-time
- Key-value access by ID
- Simple queries (get by workspace, search by content)

**What Xavier does NOT use:**
- SurrealQL query language
- Complex relational features
- Schema validation
- Multi-tenancy (we handle this in app code)
- Distributed/clustering features

---

### Three Options

#### Option A: Keep SurrealDB (Recommended Near-Term)

**Pros:**
- Stable, production-tested storage
- WebSocket protocol is convenient
- Active development

**Cons:**
- Large dependency (compiles 3+ minutes)
- Complex feature set we're not using
- License considerations for commercial use
- Protocol is proprietary to SurrealDB

**Verdict:** Use for now, architect for migration.

---

#### Option B: Replace with SQLite + App Logic

**Pros:**
- `rusqlite` is already in `code-graph`
- Single dependency, well-understood
- No network protocol to maintain
- MIT licensed, no commercial concerns

**Cons:**
- Lose WebSocket real-time capabilities (but do we need them?)
- Need to reimplement what SurrealDB gives us for free
- SQLite doesn't handle concurrent writes as well (but fine for single-writer)
- No built-in search (we'd need to build on top)

**Migration Path:**
```rust
// Instead of SurrealStore:
pub struct SqliteMemoryStore {
    conn: Connection,  // from rusqlite
}

// Implement the same MemoryStore trait
#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn get(&self, ws: &str, id: &str) -> Result<Option<MemoryRecord>> {
        // SQL: SELECT * FROM memories WHERE workspace_id = ? AND id = ?
    }
}
```

**Verdict:** Viable medium-term option. 2-3 weeks of work.

---

#### Option C: Build Custom Protocol (Long-Term)

**Concept: "Xavier Protocol"**

Design a simple wire protocol for our specific needs:

```
// Message format (JSON over WebSocket or TCP):
{
    "type": "get_memory",
    "workspace": "ws_abc",
    "id": "mem_123"
}

{
    "type": "memory_result",
    "data": { ... memory record ... }
}
```

**This is a major undertaking.** Not recommended unless:
1. We have a team dedicated to storage development
2. SurrealDB's direction doesn't match our needs
3. We have specific IP to protect in the storage layer

**Verdict:** Interesting for long-term. Not for 2026.

---

### Recommended SurrealDB Strategy

```
2026 (now):     Keep SurrealDB, focus on application IP
                - Memory graph, belief system, embeddings
                - Agent runtime, task scheduling
                - API design, user experience

Mid-2026:       Evaluate SQLite migration
                - If SurrealDB adds unwanted complexity
                - If license concerns grow
                - If we need simpler deployment

Long-term:      Consider custom protocol only if
                - We have dedicated storage team
                - We need specific protocol features
                - We want full stack ownership
```

**The core insight:** Storage is infrastructure, not competitive moat. What matters is:
- The memory model (semantic, episodic, procedural)
- The belief system (Bayesian updates, conflict resolution)
- The embedding pipeline (chunking, indexing, retrieval)
- The agent runtime (System 1/2/3, optimization)

These are all application-layer concerns. SurrealDB is just the persistence layer.

---

## Quick Reference Tables

### Dependency Status Summary

| Dependency | Category | Effort | Proprietary? | Priority |
|------------|----------|--------|-------------|----------|
| `tokio` | KEEP | — | No | — |
| `axum` | KEEP | — | No | — |
| `serde` | KEEP | — | No | — |
| `serde_json` | KEEP | — | No | — |
| `rusqlite` | KEEP | — | No | (code-graph) |
| `tree-sitter-*` | KEEP | — | No | (code-graph) |
| `anyhow` | KEEP | — | No | — |
| `reqwest` | KEEP | — | No | — |
| `clap` | KEEP | — | No | — |
| `ratatui` + `crossterm` | KEEP | — | No | (TUI only) |
| `tracing` | KEEP | — | No | — |
| `parking_lot` | KEEP | — | No | — |
| `hex` | REPLACE | 2hr | No | P1 |
| `walkdir` | REPLACE | 4hr | No | P1 |
| `tokio-stream` | MINIMIZE | 30min | No | P1 |
| `tower-http` | REMOVE | 15min | No | P1 |
| `async-trait` | REPLACE | 2hr | No | P2 |
| `uuid` | REPLACE | 1 day | No | P2 |
| `flate2`→`miniz_oxide` | REPLACE | 1 day | No | P2 |
| `regex` | REPLACE | 2-3 days | **Yes** | P2 |
| `hyper` + `hyper-util` | REMOVE | 30min | No | P2 |
| `sha2` | REPLACE | 3-4 days | **Yes** | P3 |
| `chrono` | REPLACE | 3-5 days | **Yes** | P3 |
| `cron` | REPLACE | 2 days | No | P3 |
| `surrealdb-types` | REMOVE | 10min | No | P1 |
| `surrealdb` | EVALUATE | — | No | Ongoing |
| `tower` | MINIMIZE | 30min | No | P2 |
| `anyhow`/`thiserror` | REFACTOR | 2-3 days | No | P3 |

---

### Estimated Total Effort

| Phase | Tasks | Time |
|-------|-------|------|
| Phase 1 (Quick Wins) | 6 tasks | ~1 day |
| Phase 2 (Medium) | 5 tasks | ~5 days |
| Phase 3 (Major) | 4 tasks | ~10-14 days |
| Storage Migration | (Optional) | 2-3 weeks |

**Total if fully executed: ~3-4 weeks of focused work**

---

### Top 5 Strategic Recommendations

1. **Remove `tower-http` immediately** — it's compiled but unused, wasting 2+ minutes of build time. 15-minute change.

2. **Replace `regex` in `prompt_guard`** — This is the highest proprietary value replacement. The prompt injection detection patterns ARE our IP in the security layer. Replace the generic `regex` crate with `aho-corasick` + custom pattern matchers to own this logic.

3. **Replace `chrono` with a custom `UtcDateTime`** — This is a large, slow-to-compile crate used across 15+ files. A simple `i64` timestamp wrapper with serde support is sufficient for our needs and reduces compile times significantly.

4. **Replace `sha2` + `hex` together** — They're always used together (we hash then hex-encode). A combined `sha256_hex()` function removes both dependencies. This is used in 7+ files as a utility, not as IP.

5. **Plan the SurrealDB migration path** — Don't rush to replace SurrealDB, but architect the `MemoryStore` trait so the storage backend is swappable. This gives us optionality without the immediate cost of a full rewrite.

---

*End of Dependency Strategy Document*
