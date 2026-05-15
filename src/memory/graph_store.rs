//! SQLite storage implementation for Xavier's Belief Graph.

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;
use async_trait::async_trait;

use crate::domain::memory::graph::{GraphEntity, GraphEntityType, GraphRelationship};
use crate::domain::memory::belief::{BeliefEdge, BeliefNode};

pub const TABLE_GRAPH_NODES: &str = "graph_nodes";
pub const TABLE_GRAPH_EDGES: &str = "graph_edges";

/// Port for Graph Storage operations.
#[async_trait]
pub trait GraphStorePort: Send + Sync {
    async fn put_node(&self, workspace_id: &str, node: BeliefNode) -> Result<()>;
    async fn get_node(&self, workspace_id: &str, concept: &str) -> Result<Option<BeliefNode>>;
    async fn list_nodes(&self, workspace_id: &str) -> Result<Vec<BeliefNode>>;

    async fn put_edge(&self, workspace_id: &str, edge: BeliefEdge) -> Result<()>;
    async fn list_edges(&self, workspace_id: &str) -> Result<Vec<BeliefEdge>>;
    async fn get_edge(&self, workspace_id: &str, edge_id: &str) -> Result<Option<BeliefEdge>>;
}

pub struct SqliteGraphStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteGraphStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                normalized_name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                aliases TEXT NOT NULL DEFAULT '[]',
                description TEXT,
                trust_score REAL NOT NULL DEFAULT 0.5,
                confirmation_count INTEGER NOT NULL DEFAULT 1,
                metadata TEXT NOT NULL DEFAULT '{{}}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 0.5,
                confirmation_count INTEGER NOT NULL DEFAULT 1,
                metadata TEXT NOT NULL DEFAULT '{{}}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES {}(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES {}(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_graph_nodes_normalized ON {}(normalized_name);
            CREATE INDEX IF NOT EXISTS idx_graph_edges_source ON {}(source_id);
            CREATE INDEX IF NOT EXISTS idx_graph_edges_target ON {}(target_id);
            "#,
            TABLE_GRAPH_NODES,
            TABLE_GRAPH_EDGES,
            TABLE_GRAPH_NODES,
            TABLE_GRAPH_NODES,
            TABLE_GRAPH_NODES,
            TABLE_GRAPH_EDGES,
            TABLE_GRAPH_EDGES
        ))?;
        Ok(())
    }

    pub fn upsert_entity(&self, entity: GraphEntity) -> Result<String> {
        let conn = self.conn.lock();

        // Deduplication logic: check by normalized_name and entity_type
        let mut stmt = conn.prepare(&format!(
            "SELECT id FROM {} WHERE normalized_name = ?1 AND entity_type = ?2",
            TABLE_GRAPH_NODES
        ))?;

        let existing_id: Option<String> = stmt.query_row(
            params![entity.normalized_name, entity.entity_type.as_str()],
            |row| row.get(0)
        ).ok();

        if let Some(id) = existing_id {
            // Update existing entity
            conn.execute(
                &format!(
                    "UPDATE {} SET confirmation_count = confirmation_count + 1, trust_score = MIN(1.0, trust_score + 0.1), updated_at = ?2 WHERE id = ?1",
                    TABLE_GRAPH_NODES
                ),
                params![id, Utc::now().to_rfc3339()],
            )?;
            Ok(id)
        } else {
            // Insert new entity
            conn.execute(
                &format!(
                    "INSERT INTO {} (id, name, normalized_name, entity_type, aliases, description, trust_score, confirmation_count, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    TABLE_GRAPH_NODES
                ),
                params![
                    entity.id,
                    entity.name,
                    entity.normalized_name,
                    entity.entity_type.as_str(),
                    serde_json::to_string(&entity.aliases)?,
                    entity.description,
                    entity.trust_score,
                    entity.confirmation_count,
                    serde_json::to_string(&entity.metadata)?,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                ],
            )?;
            Ok(entity.id)
        }
    }

    pub fn upsert_relationship(&self, rel: GraphRelationship) -> Result<String> {
        let conn = self.conn.lock();

        // Check if relationship already exists
        let mut stmt = conn.prepare(&format!(
            "SELECT id FROM {} WHERE source_id = ?1 AND target_id = ?2 AND relation_type = ?3",
            TABLE_GRAPH_EDGES
        ))?;

        let existing_id: Option<String> = stmt.query_row(
            params![rel.source_id, rel.target_id, rel.relation_type.as_str()],
            |row| row.get(0)
        ).ok();

        if let Some(id) = existing_id {
            // Update existing relationship
            conn.execute(
                &format!(
                    "UPDATE {} SET confirmation_count = confirmation_count + 1, weight = MIN(1.0, weight + 0.1), updated_at = ?2 WHERE id = ?1",
                    TABLE_GRAPH_EDGES
                ),
                params![id, Utc::now().to_rfc3339()],
            )?;
            Ok(id)
        } else {
            // Insert new relationship
            conn.execute(
                &format!(
                    "INSERT INTO {} (id, source_id, target_id, relation_type, weight, confirmation_count, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    TABLE_GRAPH_EDGES
                ),
                params![
                    rel.id,
                    rel.source_id,
                    rel.target_id,
                    rel.relation_type.as_str(),
                    rel.weight,
                    rel.confirmation_count,
                    serde_json::to_string(&rel.metadata)?,
                    rel.created_at.to_rfc3339(),
                    rel.updated_at.to_rfc3339(),
                ],
            )?;
            Ok(rel.id)
        }
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<GraphEntity>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, name, normalized_name, entity_type, aliases, description, trust_score, confirmation_count, metadata, created_at, updated_at FROM {} WHERE id = ?",
            TABLE_GRAPH_NODES
        ))?;

        let entity = stmt.query_row([id], |row| {
            let entity_type_str: String = row.get(3)?;
            let aliases_str: String = row.get(4)?;
            let metadata_str: String = row.get(8)?;

            Ok(GraphEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                normalized_name: row.get(2)?,
                entity_type: GraphEntityType::parse(&entity_type_str).unwrap_or(GraphEntityType::Concept),
                aliases: serde_json::from_str(&aliases_str).unwrap_or_default(),
                description: row.get(5)?,
                trust_score: row.get(6)?,
                confirmation_count: row.get(7)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        }).ok();

        Ok(entity)
    }

    pub fn list_entities(&self) -> Result<Vec<GraphEntity>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, name, normalized_name, entity_type, aliases, description, trust_score, confirmation_count, metadata, created_at, updated_at FROM {}",
            TABLE_GRAPH_NODES
        ))?;

        let rows = stmt.query_map([], |row| {
            let entity_type_str: String = row.get(3)?;
            let aliases_str: String = row.get(4)?;
            let metadata_str: String = row.get(8)?;

            Ok(GraphEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                normalized_name: row.get(2)?,
                entity_type: GraphEntityType::parse(&entity_type_str).unwrap_or(GraphEntityType::Concept),
                aliases: serde_json::from_str(&aliases_str).unwrap_or_default(),
                description: row.get(5)?,
                trust_score: row.get(6)?,
                confirmation_count: row.get(7)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut entities = Vec::new();
        for entity in rows {
            entities.push(entity?);
        }
        Ok(entities)
    }
}

#[async_trait]
impl GraphStorePort for SqliteGraphStore {
    async fn put_node(&self, _workspace_id: &str, node: BeliefNode) -> Result<()> {
        let entity = GraphEntity {
            id: node.id,
            name: node.concept.clone(),
            normalized_name: GraphEntity::normalize(&node.concept),
            entity_type: GraphEntityType::Concept,
            aliases: Vec::new(),
            description: None,
            trust_score: node.confidence,
            confirmation_count: 1,
            metadata: std::collections::HashMap::new(),
            created_at: node.created_at,
            updated_at: node.created_at,
        };
        self.upsert_entity(entity)?;
        Ok(())
    }

    async fn get_node(&self, _workspace_id: &str, concept: &str) -> Result<Option<BeliefNode>> {
        let id = GraphEntity::normalize(concept);
        if let Some(entity) = self.get_entity(&id)? {
            return Ok(Some(BeliefNode {
                id: entity.id,
                concept: entity.name,
                confidence: entity.trust_score,
                created_at: entity.created_at,
            }));
        }
        Ok(None)
    }

    async fn list_nodes(&self, _workspace_id: &str) -> Result<Vec<BeliefNode>> {
        let entities = self.list_entities()?;
        Ok(entities.into_iter().map(|e| BeliefNode {
            id: e.id,
            concept: e.name,
            confidence: e.trust_score,
            created_at: e.created_at,
        }).collect())
    }

    async fn put_edge(&self, _workspace_id: &str, edge: BeliefEdge) -> Result<()> {
        use crate::domain::memory::graph::GraphRelationshipType;
        let rel_type = GraphRelationshipType::parse(&edge.relation_type).unwrap_or(GraphRelationshipType::RelatedTo);
        
        let rel = GraphRelationship {
            id: edge.id,
            source_id: edge.source,
            target_id: edge.target,
            relation_type: rel_type,
            weight: edge.weight,
            confirmation_count: 1,
            metadata: std::collections::HashMap::new(),
            created_at: edge.created_at,
            updated_at: edge.updated_at,
        };
        self.upsert_relationship(rel)?;
        Ok(())
    }

    async fn list_edges(&self, _workspace_id: &str) -> Result<Vec<BeliefEdge>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, source_id, target_id, relation_type, weight, confirmation_count, metadata, created_at, updated_at FROM {}",
            TABLE_GRAPH_EDGES
        ))?;

        let rows = stmt.query_map([], |row| {
            let rel_type_str: String = row.get(3)?;

            Ok(BeliefEdge {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                relation_type: rel_type_str,
                weight: row.get(4)?,
                confidence_score: row.get(4)?,
                provenance_id: "unknown".to_string(),
                contradicts_edge_id: None,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut edges = Vec::new();
        for edge in rows {
            edges.push(edge?);
        }
        Ok(edges)
    }

    async fn get_edge(&self, _workspace_id: &str, edge_id: &str) -> Result<Option<BeliefEdge>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, source_id, target_id, relation_type, weight, confirmation_count, metadata, created_at, updated_at FROM {} WHERE id = ?",
            TABLE_GRAPH_EDGES
        ))?;

        let edge = stmt.query_row([edge_id], |row| {
             let rel_type_str: String = row.get(3)?;

            Ok(BeliefEdge {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                relation_type: rel_type_str,
                weight: row.get(4)?,
                confidence_score: row.get(4)?,
                provenance_id: "unknown".to_string(),
                contradicts_edge_id: None,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        }).ok();

        Ok(edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::graph::GraphEntityType;

    #[test]
    fn test_upsert_entity_deduplication() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let store = SqliteGraphStore::new(conn).unwrap();

        let e1 = GraphEntity::new("Xavier".to_string(), GraphEntityType::Product);
        let id1 = store.upsert_entity(e1).unwrap();

        let e2 = GraphEntity::new("xavier".to_string(), GraphEntityType::Product);
        let id2 = store.upsert_entity(e2).unwrap();

        assert_eq!(id1, id2);

        let entity = store.get_entity(&id1).unwrap().unwrap();
        assert_eq!(entity.confirmation_count, 2);
    }
}
