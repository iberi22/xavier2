# Xavier Memory Manager - Intelligent Memory Lifecycle Management

## Overview

The Xavier Memory Manager provides autonomous memory management capabilities that optimize storage, maintain relevance, and prevent memory overflow. It implements a biologically-inspired memory model with prioritization, decay, consolidation, and intelligent forgetting.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Memory Manager                            │
├─────────────────────────────────────────────────────────────┤
│  Prioritization  │   Decay   │  Consolidation  │  Eviction │
│  ─────────────   │   ─────   │  ────────────    │  ─────── │
│  • Critical      │ • Time-   │  • Deduplication │  • Low Q  │
│  • High          │   based   │  • Similarity   │  • High   │
│  • Medium        │ • Access  │  • Merging      │    Age    │
│  • Low           │   freq    │                 │  • Over   │
│  • Ephemeral     │ • Priority│                 │    Limit  │
│                  │   decay   │                 │           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │   vec Memory Store   │
                    │   (SQLite + sqlite-  │
                    │    vec + FTS5 + KG)  │
                    └─────────────────────┘
```

> **Backend:** `vec` — SQLite with sqlite-vec (vector search), FTS5 (full-text search), RRF fusion, and Knowledge Graph support. WAL mode enabled with mmap 256MB for high-performance concurrent reads/writes.

## Memory Priority System

Memories are classified into 5 priority levels that determine retention policy:

| Priority   | Description                      | Max Age    | Decay Rate | Min Relevance |
|------------|----------------------------------|------------|------------|---------------|
| **Critical** | BELA's profile, client data   | 10 years   | None (1.0) | 0.0 (never)  |
| **High**     | Project status, tech decisions | 1 year    | 2%/day     | 0.1           |
| **Medium**   | Operations, cron jobs         | 90 days   | 5%/day     | 0.2           |
| **Low**       | Raw logs, temp data            | 14 days   | 15%/day    | 0.3           |
| **Ephemeral** | Cache, temporary data         | 1 day     | 50%/day    | 0.5           |

### Setting Priority

Priority is set via metadata when creating a memory:

```rust
// Via TypedMemoryPayload
TypedMemoryPayload {
    metadata: serde_json::json!({
        "memory_priority": "critical"  // critical|high|medium|low|ephemeral
    }),
    ...
}
```

## Memory Quality Scoring

The quality score is a composite metric (0-1) used for eviction decisions:

```
Overall = 0.40×Relevance + 0.25×Accuracy + 0.20×Freshness + 0.15×Completeness
```

### Components

- **Relevance Score (40%)**: Based on access frequency + priority boost
- **Accuracy Score (25%)**: Based on belief graph verification status
- **Freshness Score (20%)**: Based on days since last access (older = lower)
- **Completeness Score (15%)**: Based on metadata field coverage

### Quality Buckets

| Bucket | Score Range | Action |
|--------|-------------|--------|
| High   | ≥ 0.7       | Retain indefinitely |
| Medium | 0.4 - 0.7  | Standard retention |
| Low    | < 0.4       | Eviction candidate |
| Critical | Any      | Never auto-evict |

## Decay Function

Memories decay over time using exponential decay:

```
relevance(t) = relevance₀ × decay_base^(days_since_access)
```

Where `decay_base` depends on priority:
- Critical: 1.00 (no decay)
- High: 0.98 (2% per day)
- Medium: 0.95 (5% per day)
- Low: 0.85 (15% per day)
- Ephemeral: 0.50 (50% per day)

### Example Decay Progression (Medium Priority)

| Day | Relevance |
|-----|-----------|
| 0   | 1.00      |
| 7   | 0.70      |
| 14  | 0.49      |
| 30  | 0.21      |
| 60  | 0.05      |

## Memory Consolidation

Consolidation removes duplicate and near-duplicate memories:

1. **Signature Generation**: Creates normalized content hash
2. **Duplicate Detection**: Groups memories with identical signatures
3. **Retention Policy**: Keeps most recent, archives older duplicates
4. **Metadata Merging**: Combines evidence from all duplicates

### Consolidation Signature

```rust
signature = hash(normalized_content_length + kind + priority)
```

## Intelligent Forgetting/Eviction

Eviction occurs when:
1. **Quality Threshold**: Overall score < configured threshold (default: 0.25)
2. **Age Limit**: Memory exceeds priority's max age
3. **Storage Limit**: Total storage exceeds limit (default: 500MB)
4. **Manual Request**: User explicitly requests eviction by priority

### Eviction Order (when over limit)

1. Ephemeral memories first
2. Low quality memories by priority (Critical last)
3. Oldest first within same priority

## API Endpoints

### Memory Decay
```bash
POST /memory/decay
```
Applies time-based decay to all memories. Returns documents affected.

### Memory Consolidate
```bash
POST /memory/consolidate
```
Merges duplicate and similar memories. Returns bytes freed.

### Memory Quality
```bash
GET /memory/quality?threshold=0.3
```
Lists memories below quality threshold.

### Memory Evict
```bash
DELETE /memory/evict?priority=low
DELETE /memory/evict  # Evicts by quality threshold
```
Evicts memories by priority or quality.

### Memory Stats
```bash
GET /memory/stats
```
Returns comprehensive memory statistics.

### Memory Manage (Auto)
```bash
POST /memory/manage
```
Runs full auto-management cycle: decay → consolidate → evict.

## Configuration

```rust
pub struct MemoryManagerConfig {
    pub max_documents: usize,              // 10,000 default
    pub max_storage_bytes: u64,            // 500MB default
    pub quality_threshold: f32,            // 0.25 default
    pub auto_decay_enabled: bool,          // true
    pub auto_consolidate_enabled: bool,    // true
    pub auto_evict_enabled: bool,         // true
    pub global_decay_factor: f32,           // 0.97
    pub auto_manage_interval_hours: u32,    // 24 hours
    pub compression_threshold_bytes: usize, // 2KB
}
```

## SWAL Operations Benchmark Impact

The memory manager directly impacts the SWAL-LoCoMo benchmark scores:

### Recall (Current: 88.1% → Target: 100%)
- **Critical memories never evicted** → BELA's profile, client data always available
- **Decay preserves relevance ordering** → Most relevant memories rank highest
- **Consolidation removes noise** → Fewer duplicate/irrelevant entries dilute results
- **RRF search** → Vector + FTS5 fusion catches more relevant results than either alone

### Precision (Current: 3.71/5 → Target: 4.0+)
- **Quality scoring prioritizes high-value memories** → Better recall of factual content
- **Freshness tracking** → Recent/accurate memories rank above stale ones
- **Priority-based boosting** → Critical/BELA-related content surfaces first
- **Knowledge Graph** → Structured entity relationships sharpen contextual recall

### FPR (Current: target <20%)
- **Ephemeral memories auto-expire** → Temporary cache data doesn't pollute long-term
- **Low-quality eviction** → Unverified/noisy memories removed proactively
- **Storage limit enforcement** → Prevents unbounded growth diluting signal

## Usage Examples

### Manual Curation
```bash
# Apply decay to all memories
curl -X POST http://localhost:8003/memory/decay

# Consolidate duplicates
curl -X POST http://localhost:8003/memory/consolidate

# Find low quality memories
curl http://localhost:8003/memory/quality?threshold=0.25

# Evict low priority memories
curl -X DELETE "http://localhost:8003/memory/evict?priority=low"

# Full auto-management
curl -X POST http://localhost:8003/memory/manage
```

### SWAL Benchmark Optimization
```bash
# Before benchmark: Run full optimization
curl -X POST http://localhost:8003/memory/manage

# Verify memory stats
curl http://localhost:8003/memory/stats | jq '.low_quality_count, .ephemeral_count'
```

## Integration with Knowledge Graph

The memory manager integrates with the Knowledge Graph for accuracy scoring:

```rust
// KG-verified entities boost accuracy score
if let Some(kg) = &self.knowledge_graph {
    let verified = kg.is_verified(&doc_id).await;
    accuracy_score = if verified { 1.0 } else { 0.5 };
}
```

## SWAL-LoCoMo Specific Optimizations

For the SWAL operations benchmark:

1. **BELA Profile Memories**: Always tagged as `critical` priority
2. **Client Data**: Tagged as `critical`
3. **LoCoMo Conversations**: Tagged as `high` priority with longer retention
4. **Benchmark Artifacts**: Tagged as `ephemeral`, auto-deleted after benchmark
5. **KG Entities**: Decisions and agent actions stored as typed entities for fast traversal

## Error Handling

- **Never delete Critical memories** even on storage pressure
- **Archive instead of delete** when possible (future recovery)
- **Graceful degradation** if Knowledge Graph unavailable
- **Logging on all eviction decisions** for audit trail

## Future Enhancements

- [ ] LLM-based summarization for compression
- [ ] Semantic deduplication (beyond signature matching)
- [ ] Predictive eviction based on access patterns
- [ ] Distributed memory management across workspaces
- [ ] Memory importance learning from feedback
