use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    memory::entity_graph::{EntityNeighbors, GraphDirection},
    workspace::WorkspaceContext,
};

#[derive(Debug, Deserialize)]
pub struct GraphEntityQuery {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub relation_types: Option<Vec<String>>,
    #[serde(default)]
    pub direction: Option<GraphDirection>,
}

#[derive(Debug, Deserialize)]
pub struct GraphRelationsQuery {
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub relation_types: Option<Vec<String>>,
    #[serde(default)]
    pub direction: Option<GraphDirection>,
}

#[derive(Debug, Serialize)]
pub struct GraphEntityResponse {
    pub status: String,
    pub entity: crate::memory::entity_graph::EntityRecord,
    pub incoming: Vec<crate::memory::entity_graph::EntityRelationRecord>,
    pub outgoing: Vec<crate::memory::entity_graph::EntityRelationRecord>,
    pub traversal: Vec<crate::memory::entity_graph::TraversalStep>,
}

#[derive(Debug, Serialize)]
pub struct GraphRelationsResponse {
    pub status: String,
    pub entity_id: Option<String>,
    pub direction: GraphDirection,
    pub max_depth: usize,
    pub total_relations: usize,
    pub relations: Vec<crate::memory::entity_graph::EntityRelationRecord>,
    pub traversal: Vec<crate::memory::entity_graph::TraversalStep>,
}

fn default_max_depth() -> usize {
    2
}

pub async fn memory_graph_entity(
    Extension(workspace): Extension<WorkspaceContext>,
    Path(entity_id): Path<String>,
    Query(query): Query<GraphEntityQuery>,
) -> impl IntoResponse {
    let direction = query.direction.unwrap_or_default();
    let relation_types = query.relation_types.as_deref();
    match workspace
        .workspace
        .entity_graph
        .entity_neighbors(&entity_id, query.max_depth, relation_types, direction)
        .await
    {
        Ok(EntityNeighbors {
            entity,
            incoming,
            outgoing,
            traversal,
        }) => Json(GraphEntityResponse {
            status: "ok".to_string(),
            entity,
            incoming,
            outgoing,
            traversal,
        })
        .into_response(),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "entity_id": entity_id,
        }))
        .into_response(),
    }
}

pub async fn memory_graph_relations(
    Extension(workspace): Extension<WorkspaceContext>,
    Query(query): Query<GraphRelationsQuery>,
) -> impl IntoResponse {
    let direction = query.direction.unwrap_or(GraphDirection::Both);
    let relation_types = query.relation_types.as_deref();

    if let Some(entity_id) = query.entity_id {
        match workspace
            .workspace
            .entity_graph
            .relations_for_entity(&entity_id, query.max_depth, relation_types, direction)
            .await
        {
            Ok(view) => Json(GraphRelationsResponse {
                status: "ok".to_string(),
                entity_id: view.entity_id,
                direction: view.direction,
                max_depth: view.max_depth,
                total_relations: view.total_relations,
                relations: view.relations,
                traversal: view.traversal,
            })
            .into_response(),
            Err(error) => Json(serde_json::json!({
                "status": "error",
                "message": error.to_string(),
                "entity_id": entity_id,
            }))
            .into_response(),
        }
    } else {
        let relations = workspace.workspace.entity_graph.all_relations().await;
        Json(GraphRelationsResponse {
            status: "ok".to_string(),
            entity_id: None,
            direction,
            max_depth: query.max_depth,
            total_relations: relations.len(),
            relations,
            traversal: Vec::new(),
        })
        .into_response()
    }
}
