# PHASE 1: Embeddings Layer

## Context

xavier currently uses keyword search (sqlite FTS5) and vector search (sqlite-vec), but without semantic embeddings. The search only matches exact words or near-exact phrases, not meaning.

**Why this matters:** Without embeddings, searching for "vehicle transportation" won't find memories about "cars", "automobiles", or "driving" even if they contain the same information.

## Problem Statement

Current search is limited to:
- Keyword matching (FTS5) - matches words exactly
- Vector similarity - but vectors are generated externally or missing

We need to generate embeddings for all memories to enable **semantic search** - understanding meaning, not just words.

## Technical Approach

### 1. Create Embedder Trait

**File:** `src/embedding/mod.rs` (NEW)

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embedding vector for text
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}

/// Embedding provider configuration
#[derive(Clone)]
pub enum EmbedderConfig {
    OpenAI {
        api_key: String,
        model: String,  // default: "text-embedding-3-small"
        endpoint: String,
    },
    MiniMax {
        api_key: String,
        model: String,
    },
    Local {
        model_path: String,  // ONNX/shrimp model path
    },
}

impl EmbedderConfig {
    pub fn from_env() -> Self {
        let provider = std::env::var("XAVIER_EMBEDDER").unwrap_or_default();

        match provider.as_str() {
            "openai" => Self::OpenAI {
                api_key: std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY required"),
                model: std::env::var("XAVIER_EMBEDDING_MODEL")
                    .unwrap_or("text-embedding-3-small".into()),
                endpoint: "https://api.openai.com/v1/embeddings".into(),
            },
            "minimax" => Self::MiniMax {
                api_key: std::env::var("MINIMAX_API_KEY").expect("MINIMAX_API_KEY required"),
                model: "embo-01".into(),
            },
            "local" => Self::Local {
                model_path: std::env::var("XAVIER_LOCAL_EMBEDDING_MODEL")
                    .expect("XAVIER_LOCAL_EMBEDDING_MODEL required"),
            },
            _ => Self::OpenAI {
                api_key: std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY required"),
                model: "text-embedding-3-small".into(),
                endpoint: "https://api.openai.com/v1/embeddings".into(),
            },
        }
    }
}
```

### 2. Implement OpenAI Embedder

**File:** `src/embedding/openai.rs` (NEW)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAIEmbedder {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait::async_trait]
impl super::Embedder for OpenAIEmbedder {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, super::EmbeddingError> {
        let request = EmbeddingRequest {
            input: text.to_string(),
            model: self.model.clone(),
        };

        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| super::EmbeddingError::NetworkError(e.to_string()))?;

        let body: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| super::EmbeddingError::ParseError(e.to_string()))?;

        Ok(body.data.first()
            .map(|d| d.embedding.clone())
            .unwrap_or_default())
    }

    fn dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        }
    }
}
```

### 3. Modify Memory Storage

**File:** `src/memory/storage.rs` (MODIFY)

Add vector field to MemoryDoc:

```rust
pub struct MemoryDoc {
    pub id: String,
    pub content: String,
    pub content_vector: Option<Vec<f32>>,  // ADD THIS
    pub path: String,
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub importance: f32,  // default 0.5
}
```

### 4. Modify add_memory API

**File:** `src/api/memory.rs` (MODIFY)

```rust
pub async fn add_memory(
    State(state): State<AppState>,
    Json(payload): Json<AddMemoryRequest>,
) -> Result<Json<AddMemoryResponse>, StatusCode> {
    // Generate embedding
    let content_vector = state.embedder.encode(&payload.content).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let doc = MemoryDoc {
        id: ulid::Ulid::new().to_string(),
        content: payload.content,
        content_vector: Some(content_vector),
        path: payload.path.unwrap_or_default(),
        metadata: payload.metadata.unwrap_or_default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        importance: 0.5,
    };

    // Store in sqlite-vec
    if let Some(vector) = &doc.content_vector {
        state.vec_store.insert(&doc.content, vector, &doc.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Store in DB
    state.db.insert_memory(&doc)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AddMemoryResponse {
        id: doc.id,
        status: "ok".into(),
        embedding_generated: doc.content_vector.is_some(),
    }))
}
```

### 5. Dependencies

**File:** `Cargo.toml` (MODIFY)

Add:
```toml
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json"] }
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/embedding/mod.rs` | CREATE | Embedder trait + config |
| `src/embedding/openai.rs` | CREATE | OpenAI embedder implementation |
| `src/embedding/minimax.rs` | CREATE | MiniMax embedder (future) |
| `src/memory/storage.rs` | MODIFY | Add content_vector field |
| `src/api/memory.rs` | MODIFY | Generate embedding on add |
| `src/state.rs` | MODIFY | Add embedder to AppState |
| `src/main.rs` | MODIFY | Initialize embedder from env |
| `Cargo.toml` | MODIFY | Add async-trait, reqwest |

## Acceptance Criteria

1. **Embedding generation:** When `/memory/add` is called, an embedding is generated and stored
2. **Vector storage:** Vectors are stored in sqlite-vec with the memory ID as key
3. **Config from env:** Provider selected via `XAVIER_EMBEDDER=openAI|minimax|local`
4. **Error handling:** Graceful fallback if API fails (store without vector)
5. **Tests:** `cargo test --lib test_embedding*` passes

## Verification Commands

```bash
# Test embedding generation
OPENAI_API_KEY=sk-... cargo test --lib test_embedding

# Test API integration
curl -X POST http://localhost:8003/memory/add \
  -H "Content-Type: application/json" \
  -d '{"content":"test memory for embedding"}'

# Verify vector stored
curl -X POST http://localhost:8003/memory/search \
  -d '{"query":"test"}'
```

## Priority

**🔴 CRITICAL** - Blocking for Phase 2 (Reranking) and Phase 3 (Memory Graph)

---

*Issue created: 2026-04-15*
