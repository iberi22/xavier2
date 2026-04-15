# PHASE 4: Memory Consolidation

## Context

Memories accumulate over time, but not all memories are equally important. A transient thought about "what did I have for lunch yesterday" should decay, while information about "BELA is the developer of SWAL" should be reinforced.

**Why this matters:**
- Prevents memory from being flooded with low-value information
- Reinforces important facts through repeated access
- Applies importance decay to reduce noise
- Creates a "learning" effect where frequently accessed memories become more prominent

## Problem Statement

Current system treats all memories equally:
- No importance scoring
- No decay mechanism
- No consolidation during idle time
- No way to "forget" transient information

We need **consolidation** - a background process that:
1. Replays memories and regenerates embeddings
2. Compares new embeddings with stored ones
3. Updates importance based on reinforcement
4. Applies decay to low-importance memories
5. Archives or deletes expired memories

## Technical Approach

### 1. Consolidation Module

**File:** `src/consolidation/mod.rs` (NEW)

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, Instant};
use crate::state::AppState;
use crate::memory::storage::MemoryDoc;

pub struct ConsolidationTask {
    interval_hours: u64,
    batch_size: usize,
    similarity_threshold: f32,
    decay_rate: f32,
    min_importance_for_decay: f32,
}

impl Default for ConsolidationTask {
    fn default() -> Self {
        Self {
            interval_hours: 1,  // Run every hour
            batch_size: 50,
            similarity_threshold: 0.85,
            decay_rate: 0.95,  // Multiply importance by this each cycle
            min_importance_for_decay: 0.3,
        }
    }
}

impl ConsolidationTask {
    pub async fn run(&self, state: &AppState) -> Result<ConsolidationStats> {
        let start = Instant::now();
        let mut stats = ConsolidationStats::default();
        
        // 1. Select memories for consolidation
        let memories = state.db.select_memories_for_consolidation(
            self.batch_size
        ).await?;
        
        stats.selected = memories.len();
        
        for memory in memories {
            // 2. Regenerate embedding
            let new_vector = match state.embedder.encode(&memory.content).await {
                Ok(v) => v,
                Err(e) => {
                    stats.embedding_errors += 1;
                    continue;
                }
            };
            
            // 3. Compare with stored embedding
            if let Some(stored_vector) = &memory.content_vector {
                let similarity = cosine_similarity(stored_vector, &new_vector);
                
                if similarity < self.similarity_threshold {
                    // Memory has drifted - mark for review
                    state.db.mark_for_review(&memory.id, similarity).await?;
                    stats.revised += 1;
                } else {
                    // Memory stable - reinforce importance
                    state.db.increment_importance(&memory.id, 0.1).await?;
                    stats.reinforced += 1;
                }
            }
            
            // 4. Apply decay
            if memory.importance >= self.min_importance_for_decay {
                state.db.apply_decay(&memory.id, self.decay_rate).await?;
                stats.decayed += 1;
            }
            
            // 5. Check for expiration
            if memory.expires_at.map(|e| e < Utc::now()).unwrap_or(false) {
                state.db.archive_memory(&memory.id).await?;
                stats.expired += 1;
            }
            
            stats.processed += 1;
        }
        
        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }
    
    pub async fn start_scheduler(self: Arc<Self>, state: AppState) {
        let mut ticker = interval(Duration::hours(self.interval_hours));
        
        loop {
            ticker.tick().await;
            
            tracing::info!("Starting memory consolidation");
            match self.run(&state).await {
                Ok(stats) => {
                    tracing::info!(
                        "Consolidation complete: processed={} reinforced={} revised={} decayed={}",
                        stats.processed, stats.reinforced, stats.revised, stats.decayed
                    );
                }
                Err(e) => {
                    tracing::error!("Consolidation failed: {}", e);
                }
            }
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}

#[derive(Default, Debug, Clone)]
pub struct ConsolidationStats {
    pub selected: usize,
    pub processed: usize,
    pub reinforced: usize,
    pub revised: usize,
    pub decayed: usize,
    pub expired: usize,
    pub embedding_errors: usize,
    pub duration_ms: u64,
}
```

### 2. Database Operations

**File:** `src/db/consolidation.rs` (NEW)

```rust
use crate::error::DatabaseError;

pub async fn select_memories_for_consolidation(
    db: &Database,
    batch_size: usize,
) -> Result<Vec<MemoryDoc>, DatabaseError> {
    // Select memories that:
    // 1. Haven't been consolidated recently (>1 hour)
    // 2. Are not marked for review
    // 3. Sort by importance (process high-importance first)
    
    let query = r#"
        SELECT id, content, content_vector, path, metadata,
               importance, created_at, updated_at, expires_at
        FROM memories
        WHERE last_consolidated_at < datetime('now', '-1 hour')
          AND review_status != 'pending'
        ORDER BY importance DESC
        LIMIT ?
    "#;
    
    db.query_many(query, &[&batch_size])
        .await
        .map_err(|e| DatabaseError::QueryError(e.to_string()))
}

pub async fn mark_for_review(
    db: &Database,
    memory_id: &str,
    similarity: f32,
) -> Result<(), DatabaseError> {
    let query = r#"
        UPDATE memories 
        SET review_status = 'pending',
            review_reason = ?,
            last_consolidated_at = CURRENT_TIMESTAMP
        WHERE id = ?
    "#;
    
    db.execute(query, &[&similarity.to_string(), &memory_id])
        .await
        .map_err(|e| DatabaseError::UpdateError(e.to_string()))
}

pub async fn increment_importance(
    db: &Database,
    memory_id: &str,
    delta: f32,
) -> Result<(), DatabaseError> {
    let query = r#"
        UPDATE memories 
        SET importance = MIN(1.0, importance + ?),
            last_consolidated_at = CURRENT_TIMESTAMP
        WHERE id = ?
    "#;
    
    db.execute(query, &[&delta.to_string(), &memory_id])
        .await
        .map_err(|e| DatabaseError::UpdateError(e.to_string()))
}

pub async fn apply_decay(
    db: &Database,
    memory_id: &str,
    decay_rate: f32,
) -> Result<(), DatabaseError> {
    let query = r#"
        UPDATE memories 
        SET importance = importance * ?,
            last_consolidated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND importance >= 0.3
    "#;
    
    db.execute(query, &[&decay_rate.to_string(), &memory_id])
        .await
        .map_err(|e| DatabaseError::UpdateError(e.to_string()))
}

pub async fn archive_memory(
    db: &Database,
    memory_id: &str,
) -> Result<(), DatabaseError> {
    // Move to archive table and delete from main
    let archive_query = r#"
        INSERT INTO memories_archive 
        SELECT *, CURRENT_TIMESTAMP as archived_at
        FROM memories WHERE id = ?
    "#;
    
    db.execute(archive_query, &[&memory_id])
        .await
        .map_err(|e| DatabaseError::ArchiveError(e.to_string()))?;
    
    let delete_query = "DELETE FROM memories WHERE id = ?";
    db.execute(delete_query, &[&memory_id])
        .await
        .map_err(|e| DatabaseError::DeleteError(e.to_string()))
}
```

### 3. Database Schema Changes

**File:** `src/db/schema.sql` (MODIFY)

```sql
-- Add consolidation-related columns to memories
ALTER TABLE memories ADD COLUMN importance REAL DEFAULT 0.5;
ALTER TABLE memories ADD COLUMN last_consolidated_at TIMESTAMP;
ALTER TABLE memories ADD COLUMN review_status TEXT DEFAULT 'none';
ALTER TABLE memories ADD COLUMN review_reason TEXT;
ALTER TABLE memories ADD COLUMN archived_at TIMESTAMP;

-- Archive table for expired/deleted memories
CREATE TABLE IF NOT EXISTS memories_archive (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_vector BLOB,
    path TEXT,
    metadata TEXT,
    importance REAL,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    expires_at TIMESTAMP,
    archived_at TIMESTAMP NOT NULL
);

-- Index for consolidation queries
CREATE INDEX IF NOT EXISTS idx_memories_consolidation 
ON memories(last_consolidated_at, importance);
```

### 4. API Endpoints

**File:** `src/api/consolidation.rs` (NEW)

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::state::AppState;
use crate::consolidation::ConsolidationTask;

#[derive(Deserialize)]
pub struct ConsolidateRequest {
    pub batch_size: Option<usize>,
    pub force: Option<bool>,  // Skip schedule, run immediately
}

#[derive(Serialize)]
pub struct ConsolidateResponse {
    pub status: String,
    pub stats: ConsolidationStats,
}

#[derive(Serialize)]
pub struct ConsolidationStatus {
    pub last_run: Option<String>,
    pub total_processed: usize,
    pub next_run: Option<String>,
    pub schedule_interval_hours: u64,
}

pub async fn consolidate(
    State(state): State<AppState>,
    Json(request): Json<ConsolidateRequest>,
) -> Result<Json<ConsolidateResponse>, StatusCode> {
    let batch_size = request.batch_size.unwrap_or(50);
    
    let task = ConsolidationTask {
        interval_hours: 1,
        batch_size,
        similarity_threshold: 0.85,
        decay_rate: 0.95,
        min_importance_for_decay: 0.3,
    };
    
    let stats = task.run(&state).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(ConsolidateResponse {
        status: "ok".to_string(),
        stats,
    }))
}

pub async fn consolidation_status(
    State(state): State<AppState>,
) -> Result<Json<ConsolidationStatus>, StatusCode> {
    // Get last run info from database
    let last_run = state.db.get_last_consolidation_time().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db.get_consolidation_total().await
        .unwrap_or(0);
    
    Ok(Json(ConsolidationStatus {
        last_run,
        total_processed: total,
        next_run: last_run.map(|lr| {
            // TODO: calculate next run time
            format!("{} + 1 hour", lr)
        }),
        schedule_interval_hours: 1,
    }))
}
```

### 5. Modify Main to Start Scheduler

**File:** `src/main.rs` (MODIFY)

```rust
use crate::consolidation::ConsolidationTask;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup ...
    
    // Initialize embedder
    let embedder = EmbedderConfig::from_env().build()?;
    
    // Initialize app state
    let state = AppState {
        embedder,
        // ... other fields ...
    };
    
    // Start consolidation scheduler in background
    let consolidation_task = Arc::new(ConsolidationTask::default());
    let state_for_scheduler = state.clone();
    tokio::spawn(async move {
        consolidation_task.start_scheduler(state_for_scheduler).await;
    });
    
    // ... rest of main ...
}
```

## Importance Score System

| Score | Meaning | Behavior |
|-------|---------|----------|
| 0.8 - 1.0 | Critical | Never decay, always reinforced |
| 0.5 - 0.8 | Important | Slow decay |
| 0.3 - 0.5 | Normal | Standard decay |
| 0.0 - 0.3 | Low value | Fast decay, eligible for archive |

## Consolidation Triggers

1. **Scheduled:** Every hour automatically
2. **Manual:** `POST /memory/consolidate` with `force: true`
3. **API threshold:** When memory count exceeds threshold
4. **Idle trigger:** When system is idle (future enhancement)

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/consolidation/mod.rs` | CREATE | Consolidation task |
| `src/consolidation/db.rs` | CREATE | DB operations for consolidation |
| `src/api/consolidation.rs` | CREATE | API endpoints |
| `src/db/schema.sql` | MODIFY | Add importance, consolidation columns |
| `src/state.rs` | MODIFY | Add consolidation task |
| `src/main.rs` | MODIFY | Start scheduler |
| `Cargo.toml` | MODIFY | Add tokio (already present) |

## Acceptance Criteria

1. **Scheduled runs:** Consolidation runs automatically every hour
2. **Reinforcement:** High-similarity memories increase in importance
3. **Revision marking:** Drifted memories marked for review
4. **Decay:** Low-importance memories decay over time
5. **Archiving:** Expired memories moved to archive
6. **Manual trigger:** `POST /memory/consolidate` works
7. **Status endpoint:** `GET /memory/consolidation/status` returns info
8. **Tests:** `cargo test --lib test_consolidation*` passes

## Verification Commands

```bash
# Trigger consolidation manually
curl -X POST http://localhost:8003/memory/consolidate \
  -H "Content-Type: application/json" \
  -d '{"batch_size":10,"force":true}'

# Check consolidation status
curl -X GET http://localhost:8003/memory/consolidation/status

# View memories due for consolidation
curl -X GET http://localhost:8003/memory/pending-review

# Check importance scores
curl -X POST http://localhost:8003/memory/search \
  -d '{"query":"BELA","limit":5}' | jq '.results[].metadata.importance'

# View archived memories
curl -X GET http://localhost:8003/memory/archived
```

## Metrics to Track

```json
{
  "consolidation_stats": {
    "total_processed": 1500,
    "reinforced": 1200,
    "revised": 150,
    "decayed": 148,
    "expired": 2,
    "avg_importance": 0.65
  }
}
```

## Priority

**🟡 MEDIUM** - Depends on Phase 1 (Embeddings)

---

*Issue created: 2026-04-15*
