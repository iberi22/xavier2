# PHASE 3: Memory Graph

## Context

xavier2 stores memories as flat documents, but human memory is structured with entities and relationships. "BELA works at SWAL" connects two entities (BELA, SWAL) with a relation (works_at).

**Why this matters:** 
- Entity tracking enables queries like "What does BELA know?" or "Who works at SWAL?"
- Relationship graphs reveal connections that keyword search would miss
- Enables sophisticated reasoning about connected concepts

## Problem Statement

Current system:
- Memories are flat key-value documents
- No entity extraction or tracking
- No relationship modeling
- Graph traversal not supported

We need a **belief graph** that tracks:
- Entities (people, organizations, concepts)
- Relations between entities
- Confidence scores for beliefs
- Temporal decay of information

## Technical Approach

### 1. Database Schema Changes

**File:** `src/db/schema.sql` (MODIFY)

```sql
-- Entities table
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,  -- 'person', 'org', 'concept', 'event', 'location'
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Entity relations table
CREATE TABLE IF NOT EXISTS entity_relations (
    id TEXT PRIMARY KEY,
    from_entity TEXT NOT NULL REFERENCES entities(id),
    to_entity TEXT NOT NULL REFERENCES entities(id),
    relation_type TEXT NOT NULL,  -- 'knows', 'works_at', 'part_of', 'uses', 'related_to'
    weight REAL DEFAULT 1.0,
    provenance TEXT,  -- memory_id that established this relation
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_entity, to_entity, relation_type)
);

-- Beliefs table (expanded from existing)
CREATE TABLE IF NOT EXISTS beliefs (
    id TEXT PRIMARY KEY,
    subject_entity TEXT REFERENCES entities(id),
    predicate TEXT NOT NULL,
    object_entity TEXT REFERENCES entities(id),
    object_literal TEXT,  -- for non-entity values
    confidence REAL DEFAULT 0.5,
    source_memory TEXT REFERENCES memories(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP
);

-- Indexes for graph traversal
CREATE INDEX IF NOT EXISTS idx_entity_relations_from ON entity_relations(from_entity);
CREATE INDEX IF NOT EXISTS idx_entity_relations_to ON entity_relations(to_entity);
CREATE INDEX IF NOT EXISTS idx_entity_relations_type ON entity_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_beliefs_subject ON beliefs(subject_entity);
```

### 2. Entity Extraction

**File:** `src/memory/entity_extractor.rs` (NEW)

```rust
use regex::Regex;
use once_cell::sync::Lazy;

// Patterns for entity detection
static CAPITALIZED_WORD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\b").unwrap()
});

static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\w.+-]+@[\w-]+\.[\w.-]+").unwrap()
});

static URL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[^\s]+").unwrap()
});

#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: EntityType,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Concept,
    Email,
    Url,
    Unknown,
}

pub struct EntityExtractor;

impl EntityExtractor {
    /// Extract entities from text content
    pub fn extract(text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        
        // Extract emails
        for cap in EMAIL_PATTERN.captures_iter(text) {
            entities.push(ExtractedEntity {
                name: cap[0].to_string(),
                entity_type: EntityType::Email,
                span: cap.get(0).map(|m| (m.start(), m.end())).unwrap_or((0, 0)),
            });
        }
        
        // Extract URLs
        for cap in URL_PATTERN.captures_iter(text) {
            entities.push(ExtractedEntity {
                name: cap[0].to_string(),
                entity_type: EntityType::Url,
                span: cap.get(0).map(|m| (m.start(), m.end())).unwrap_or((0, 0)),
            });
        }
        
        // Extract capitalized words (potential persons/orgs)
        for cap in CAPITALIZED_WORD.captures_iter(text) {
            let name = cap[1].to_string();
            // Filter common words
            if !Self::is_common_word(&name) {
                entities.push(ExtractedEntity {
                    name: name.clone(),
                    entity_type: Self::guess_entity_type(&name),
                    span: cap.get(0).map(|m| (m.start(), m.end())).unwrap_or((0, 0)),
                });
            }
        }
        
        entities
    }
    
    fn is_common_word(word: &str) -> bool {
        matches!(word, "The" | "This" | "That" | "These" | "Those" | "OpenClaw" | "Windows" | "Docker")
    }
    
    fn guess_entity_type(name: &str) -> EntityType {
        // Simple heuristics
        if name.ends_with(" Inc") || name.ends_with(" Corp") || name.ends_with(" LLC") {
            EntityType::Organization
        } else if matches!(name, "SWAL" | "MIT" | "NASA") {
            EntityType::Organization
        } else if name.contains(" ") && name.chars().all(|c| c.is_uppercase() || c.is_whitespace()) {
            EntityType::Organization
        } else {
            EntityType::Person  // Default to person for capitalized names
        }
    }
}
```

### 3. Belief Graph Module

**File:** `src/memory/belief_graph.rs` (MODIFY - expand existing)

```rust
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};

pub struct BeliefGraph {
    entities: HashMap<String, Entity>,
    relations: HashMap<String, Vec<Relation>>,
    db: Database,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub belief_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub provenance: Option<String>,
}

impl BeliefGraph {
    /// Add or update entity from memory content
    pub async fn process_memory(&mut self, memory: &Memory) -> Result<Vec<Relation>> {
        let entities = EntityExtractor::extract(&memory.content);
        let mut new_relations = Vec::new();
        
        for entity in entities {
            // Upsert entity
            let entity_id = self.upsert_entity(&entity).await?;
            
            // Try to extract relations (subject-verb-object patterns)
            if let Some(relation) = self.extract_relation(&memory.content, &entity.name) {
                let rel = self.upsert_relation(entity_id, &relation).await?;
                new_relations.push(rel);
            }
        }
        
        Ok(new_relations)
    }
    
    /// BFS traversal from an entity
    pub async fn traverse(
        &self,
        start_entity: &str,
        max_depth: usize,
        relation_types: Option<&[String]>,
    ) -> Result<Vec<TraversalResult>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize, Vec<Relation>)> = VecDeque::new();
        let mut results = Vec::new();
        
        queue.push_back((start_entity.to_string(), 0, vec![]));
        
        while let Some((current, depth, path)) = queue.pop_front() {
            if depth >= max_depth || visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            // Get outgoing relations
            if let Some(relations) = self.relations.get(&current) {
                for rel in relations {
                    if let Some(types) = relation_types {
                        if !types.contains(&rel.relation_type) {
                            continue;
                        }
                    }
                    
                    results.push(TraversalResult {
                        from: current.clone(),
                        to: rel.to.clone(),
                        relation_type: rel.relation_type.clone(),
                        depth,
                        path: path.iter().chain(std::iter::once(rel)).cloned().collect(),
                    });
                    
                    queue.push_back((rel.to.clone(), depth + 1, path.iter().cloned().chain(std::iter::once(rel)).collect()));
                }
            }
        }
        
        Ok(results)
    }
    
    async fn upsert_entity(&mut self, extracted: &ExtractedEntity) -> Result<String> {
        let id = self.db.find_or_create_entity(
            &extracted.name,
            &format!("{:?}", extracted.entity_type),
        ).await?;
        
        self.entities.insert(id.clone(), Entity {
            id: id.clone(),
            name: extracted.name.clone(),
            entity_type: format!("{:?}", extracted.entity_type),
            description: None,
            belief_count: 0,
        });
        
        Ok(id)
    }
    
    fn extract_relation(&self, text: &str, subject: &str) -> Option<ExtractedRelation> {
        // Simple pattern: "X [relation_verb] Y"
        let relation_patterns = [
            (r"(\w+)\s+works?\s+at\s+(\w+)", "works_at"),
            (r"(\w+)\s+knows?\s+(\w+)", "knows"),
            (r"(\w+)\s+uses?\s+(\w+)", "uses"),
            (r"(\w+)\s+is\s+a[n]?\s+(\w+)", "is_a"),
            (r"(\w+)\s+part\s+of\s+(\w+)", "part_of"),
        ];
        
        for (pattern, rel_type) in relation_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(text) {
                    return Some(ExtractedRelation {
                        from: subject.to_string(),
                        to: caps.get(2)?.as_str().to_string(),
                        relation_type: rel_type.to_string(),
                    });
                }
            }
        }
        
        None
    }
}

#[derive(Debug)]
pub struct ExtractedRelation {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

#[derive(Debug, Serialize)]
pub struct TraversalResult {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub depth: usize,
    pub path: Vec<Relation>,
}
```

### 4. Graph API Endpoint

**File:** `src/api/graph.rs` (NEW)

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GraphQuery {
    pub entity: String,
    pub max_depth: Option<usize>,
    pub relation_types: Option<Vec<String>>,
    pub direction: Option<String>,  // "outgoing", "incoming", "both"
}

#[derive(Serialize)]
pub struct GraphResponse {
    pub entity: String,
    pub relations: Vec<RelationResponse>,
    pub total_relations: usize,
}

#[derive(Serialize)]
pub struct RelationResponse {
    pub type_: String,
    pub target: String,
    pub weight: f32,
    pub depth: usize,
}

pub async fn graph_query(
    State(state): State<AppState>,
    Json(query): Json<GraphQuery>,
) -> Result<Json<GraphResponse>, StatusCode> {
    let max_depth = query.max_depth.unwrap_or(2);
    let direction = query.direction.as_deref().unwrap_or("outgoing");
    
    let graph = &state.belief_graph;
    
    // Traverse graph
    let relations = graph.traverse(
        &query.entity,
        max_depth,
        query.relation_types.as_deref(),
    ).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Filter by direction
    let filtered: Vec<_> = relations.into_iter()
        .filter(|r| {
            match direction {
                "outgoing" => true,  // all results are from traversal
                "incoming" => false,  // TODO: implement reverse traversal
                "both" => true,
                _ => true,
            }
        })
        .collect();
    
    let response = GraphResponse {
        entity: query.entity,
        relations: filtered.iter().map(|r| RelationResponse {
            type_: r.relation_type.clone(),
            target: r.to.clone(),
            weight: 1.0,  // TODO: get from relation
            depth: r.depth,
        }).collect(),
        total_relations: filtered.len(),
    };
    
    Ok(Json(response))
}
```

### 5. Modify Memory Add to Process Entities

**File:** `src/api/memory.rs` (MODIFY)

Add entity processing after memory insertion:

```rust
pub async fn add_memory(
    State(state): State<AppState>,
    Json(payload): Json<AddMemoryRequest>,
) -> Result<Json<AddMemoryResponse>, StatusCode> {
    // ... existing code ...
    
    // Generate embedding (Phase 1)
    let content_vector = state.embedder.encode(&payload.content).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Store memory
    let doc = MemoryDoc { /* ... */ };
    state.db.insert_memory(&doc).await?;
    
    // NEW: Process entities and relations
    let relations = state.belief_graph.process_memory(&doc).await
        .unwrap_or_default();  // Don't fail on entity extraction errors
    
    Ok(Json(AddMemoryResponse {
        id: doc.id,
        status: "ok".into(),
        embedding_generated: doc.content_vector.is_some(),
        relations_extracted: relations.len(),
    }))
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/memory/entity_extractor.rs` | CREATE | Entity extraction logic |
| `src/memory/belief_graph.rs` | MODIFY | Expand with entity tracking |
| `src/api/graph.rs` | CREATE | Graph query API |
| `src/api/memory.rs` | MODIFY | Call process_memory |
| `src/db/schema.sql` | MODIFY | Add entities, relations tables |
| `src/state.rs` | MODIFY | Add BeliefGraph to AppState |
| `Cargo.toml` | MODIFY | Add regex, once_cell |

## Relation Types

| Type | Example | Description |
|------|---------|-------------|
| knows | "BELA knows Leonardo" | Person-person connection |
| works_at | "BELA works at SWAL" | Person-organization |
| part_of | "ManteniApp part_of SWAL" | Project-organization |
| uses | "BELA uses xavier2" | Person-tool/service |
| is_a | "xavier2 is_a memory system" | Object categorization |
| related_to | "ManteniApp related_to Cortex" | General relation |

## Acceptance Criteria

1. **Entity extraction:** Memories are scanned for entities on add
2. **Entity persistence:** Entities stored in database with types
3. **Relation tracking:** Relations extracted and stored
4. **Graph traversal:** BFS traversal works with depth limit
5. **API endpoint:** `POST /memory/graph` returns entity relations
6. **Tests:** `cargo test --lib test_entity*` and `test_belief*` pass

## Verification Commands

```bash
# Add memory with entities
curl -X POST http://localhost:8003/memory/add \
  -H "Content-Type: application/json" \
  -d '{"content":"BELA works at SWAL and knows Leonardo who works at Rodacenter"}'

# Query graph
curl -X POST http://localhost:8003/memory/graph \
  -H "Content-Type: application/json" \
  -d '{"entity":"BELA","max_depth":2}'

# Get all entities
curl -X GET http://localhost:8003/memory/entities

# Get entity types
curl -X GET http://localhost:8003/memory/entities/persons
```

## Priority

**🟡 MEDIUM** - Independent, can parallelize with Phase 2

---

*Issue created: 2026-04-15*
