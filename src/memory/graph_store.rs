use anyhow::Result;
use async_trait::async_trait;
use crate::domain::memory::belief::{BeliefEdge, BeliefNode};

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn put_node(&self, workspace_id: &str, node: BeliefNode) -> Result<()>;
    async fn get_node(&self, workspace_id: &str, concept: &str) -> Result<Option<BeliefNode>>;
    async fn list_nodes(&self, workspace_id: &str) -> Result<Vec<BeliefNode>>;

    async fn put_edge(&self, workspace_id: &str, edge: BeliefEdge) -> Result<()>;
    async fn list_edges(&self, workspace_id: &str) -> Result<Vec<BeliefEdge>>;
    async fn get_edge(&self, workspace_id: &str, edge_id: &str) -> Result<Option<BeliefEdge>>;
}
