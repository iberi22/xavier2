use crate::domain::memory::belief::BeliefEdge;

#[derive(Debug, Clone)]
pub struct BeliefEvaluator;

impl BeliefEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates the confidence of a new belief based on its source and context.
    pub async fn evaluate_confidence(&self, source_type: &str, content: &str) -> f32 {
        let base_confidence = match source_type {
            "verified_fact" => 0.95,
            "user_input" => 0.8,
            "inference" => 0.6,
            "observation" => 0.7,
            _ => 0.5,
        };

        // Simple heuristic: length of content and presence of certain keywords
        let content_factor = (content.len() as f32 / 100.0).clamp(0.0, 0.05);

        (base_confidence + content_factor).clamp(0.0, 1.0)
    }

    /// Identifies if a new belief contradicts existing edges in the graph.
    pub fn find_contradiction(
        &self,
        new_edge: &BeliefEdge,
        existing_edges: &[BeliefEdge],
    ) -> Option<String> {
        for existing in existing_edges {
            if existing.source == new_edge.source
                && existing.target == new_edge.target
                && existing.relation_type == new_edge.relation_type
                && existing.provenance_id != new_edge.provenance_id
            {
                // This is a competing belief for the same triple but from a different source
                return Some(existing.id.clone());
            }
        }
        None
    }
}

impl Default for BeliefEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
