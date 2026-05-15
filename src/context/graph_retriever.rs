use crate::memory::belief_graph::SharedBeliefGraph;
use crate::memory::graph_traversal::Pathfinder;

pub struct GraphRetriever {
    graph: SharedBeliefGraph,
}

impl GraphRetriever {
    pub fn new(graph: SharedBeliefGraph) -> Self {
        Self { graph }
    }

    /// Retrieves documents along the shortest path between two concepts.
    pub async fn retrieve_logic_path(&self, start: &str, end: &str) -> Vec<String> {
        let graph = self.graph.read().await;
        let pathfinder = Pathfinder::new(&graph);
        let path = pathfinder.shortest_path(start, end);

        path.into_iter()
            .filter_map(|rel| rel.source_memory_id)
            .collect()
    }

    /// Retrieves documents within a subgraph expansion from a concept.
    pub async fn retrieve_subgraph_context(&self, start: &str, depth: usize) -> Vec<String> {
        let graph = self.graph.read().await;
        let pathfinder = Pathfinder::new(&graph);
        let relations = pathfinder.k_hop_expansion(start, depth);

        let mut memory_ids: Vec<String> = relations
            .into_iter()
            .filter_map(|rel| rel.source_memory_id)
            .collect();

        memory_ids.sort();
        memory_ids.dedup();
        memory_ids
    }
}
