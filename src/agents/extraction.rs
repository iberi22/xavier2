//! Extraction pipeline for the Belief Graph.
//!
//! Provides services for extracting structured entities and relationships
//! from unstructured text while enforcing a strict schema.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::memory::graph::{
    GraphEntity, GraphEntityType, GraphRelationship, GraphRelationshipType,
};
use crate::memory::graph_store::GraphStore;

/// Structured output for entity extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: GraphEntityType,
    pub description: Option<String>,
}

/// Structured output for relationship extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub source_name: String,
    pub target_name: String,
    pub relation_type: GraphRelationshipType,
}

/// Result of the extraction process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
}

pub struct ExtractionService {
    graph_store: Arc<GraphStore>,
}

impl ExtractionService {
    pub fn new(graph_store: Arc<GraphStore>) -> Self {
        Self { graph_store }
    }

    /// Extracts entities and relationships from text and persists them.
    pub async fn process_text(&self, text: &str) -> Result<ExtractionResult> {
        // In a real implementation, this would call an LLM with a constrained JSON schema.
        // For this foundation, we implement a deterministic rule-based extractor
        // that can be later augmented with LLM calls.

        let extracted = self.extract_deterministic(text);

        // Persist extracted data
        for ent in &extracted.entities {
            let entity = GraphEntity::new(ent.name.clone(), ent.entity_type);
            self.graph_store.upsert_entity(entity)?;
        }

        for rel in &extracted.relationships {
            let source_id = self.resolve_entity_id(&rel.source_name, None)?;
            let target_id = self.resolve_entity_id(&rel.target_name, None)?;

            if let (Some(s_id), Some(t_id)) = (source_id, target_id) {
                let relationship = GraphRelationship::new(s_id, t_id, rel.relation_type);
                self.graph_store.upsert_relationship(relationship)?;
            }
        }

        Ok(extracted)
    }

    fn extract_deterministic(&self, text: &str) -> ExtractionResult {
        // Simple deterministic extraction logic for testing and baseline
        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        // Very basic "X works at Y" pattern
        if text.contains(" works at ") {
            let parts: Vec<&str> = text.split(" works at ").collect();
            if parts.len() == 2 {
                let person_name = parts[0].trim();
                let org_name = parts[1].trim().trim_matches('.');

                entities.push(ExtractedEntity {
                    name: person_name.to_string(),
                    entity_type: GraphEntityType::Person,
                    description: None,
                });

                entities.push(ExtractedEntity {
                    name: org_name.to_string(),
                    entity_type: GraphEntityType::Organization,
                    description: None,
                });

                relationships.push(ExtractedRelationship {
                    source_name: person_name.to_string(),
                    target_name: org_name.to_string(),
                    relation_type: GraphRelationshipType::WorksAt,
                });
            }
        }

        ExtractionResult {
            entities,
            relationships,
        }
    }

    fn resolve_entity_id(&self, name: &str, entity_type: Option<GraphEntityType>) -> Result<Option<String>> {
        let normalized = GraphEntity::normalize(name);
        let entities = self.graph_store.list_entities()?;

        let found = entities.iter().find(|e| {
            if let Some(et) = entity_type {
                e.normalized_name == normalized && e.entity_type == et
            } else {
                e.normalized_name == normalized
            }
        });

        Ok(found.map(|e| e.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use rusqlite::Connection;

    #[tokio::test]
    async fn test_process_text_extraction() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let graph_store = Arc::new(GraphStore::new(conn).unwrap());
        let service = ExtractionService::new(graph_store.clone());

        let text = "Alice works at Acme Corp.";
        let result = service.process_text(text).await.unwrap();

        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.relationships.len(), 1);

        let entities = graph_store.list_entities().unwrap();
        assert_eq!(entities.len(), 2);

        let alice = entities.iter().find(|e| e.name == "Alice").unwrap();
        assert_eq!(alice.entity_type, GraphEntityType::Person);

        let acme = entities.iter().find(|e| e.name == "Acme Corp").unwrap();
        assert_eq!(acme.entity_type, GraphEntityType::Organization);
    }
}
