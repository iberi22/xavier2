use crate::memory::belief_graph::{BeliefGraph, BeliefRelation};

pub struct Pathfinder<'a> {
    graph: &'a BeliefGraph,
}

impl<'a> Pathfinder<'a> {
    pub fn new(graph: &'a BeliefGraph) -> Self {
        Self { graph }
    }

    /// Finds the shortest path between two concepts using BFS.
    /// Returns a list of relations forming the path.
    pub fn shortest_path(&self, start: &str, end: &str) -> Vec<BeliefRelation> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Queue stores (current_concept, path_of_relations)
        queue.push_back((start.to_string(), Vec::new()));
        visited.insert(start.to_string());

        let relations = self.graph.get_relations();

        while let Some((current, path)) = queue.pop_front() {
            if current == end {
                return path;
            }

            for relation in &relations {
                if relation.source == current {
                    if !visited.contains(&relation.target) {
                        visited.insert(relation.target.clone());
                        let mut new_path = path.clone();
                        new_path.push(relation.clone());
                        queue.push_back((relation.target.clone(), new_path));
                    }
                }
            }
        }

        Vec::new()
    }

    /// Performs a k-hop expansion from a start concept.
    /// Returns all relations within k hops.
    pub fn k_hop_expansion(&self, start: &str, k: usize) -> Vec<BeliefRelation> {
        let mut result = Vec::new();
        let mut visited_nodes = std::collections::HashSet::new();
        let mut visited_relations = std::collections::HashSet::new();
        let mut current_layer = std::collections::HashSet::new();

        current_layer.insert(start.to_string());
        visited_nodes.insert(start.to_string());

        let all_relations = self.graph.get_relations();

        for _ in 0..k {
            let mut next_layer = std::collections::HashSet::new();
            for current in current_layer {
                for relation in &all_relations {
                    if relation.source == current {
                        if visited_relations.insert(relation.id.clone()) {
                            result.push(relation.clone());
                        }
                        if visited_nodes.insert(relation.target.clone()) {
                            next_layer.insert(relation.target.clone());
                        }
                    }
                    // For expansion we might also want to consider incoming relations if the graph is undirected,
                    // but the objective says "follows defined edges", which usually implies directed.
                    // Given belief graph is typically A -> predicate -> B, we'll stick to outgoing for now.
                }
            }
            if next_layer.is_empty() {
                break;
            }
            current_layer = next_layer;
        }

        result
    }

    /// Finds all possible paths from start to end up to max_depth.
    pub fn all_paths(&self, start: &str, end: &str, max_depth: usize) -> Vec<Vec<BeliefRelation>> {
        let mut results = Vec::new();
        let relations = self.graph.get_relations();
        self.find_all_paths_recursive(start, end, max_depth, Vec::new(), &relations, &mut results);
        results
    }

    fn find_all_paths_recursive(
        &self,
        current: &str,
        end: &str,
        depth_left: usize,
        current_path: Vec<BeliefRelation>,
        all_relations: &[BeliefRelation],
        results: &mut Vec<Vec<BeliefRelation>>,
    ) {
        if current == end {
            if !current_path.is_empty() {
                results.push(current_path);
            }
            return;
        }

        if depth_left == 0 {
            return;
        }

        for relation in all_relations {
            if relation.source == current {
                // Avoid cycles in a single path
                if !current_path.iter().any(|r| r.target == relation.target) {
                    let mut next_path = current_path.clone();
                    next_path.push(relation.clone());
                    self.find_all_paths_recursive(
                        &relation.target,
                        end,
                        depth_left - 1,
                        next_path,
                        all_relations,
                        results,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pathfinding() {
        let graph = BeliefGraph::new();
        // A -> B -> C
        graph.add_node("A".to_string(), 1.0);
        graph.add_node("B".to_string(), 1.0);
        graph.add_node("C".to_string(), 1.0);
        graph.add_node("D".to_string(), 1.0);

        graph.add_relation("A".to_string(), "B".to_string(), "related_to".to_string(), 1.0, Some("mem1".to_string()), None, None);
        graph.add_relation("B".to_string(), "C".to_string(), "related_to".to_string(), 1.0, Some("mem2".to_string()), None, None);
        graph.add_relation("A".to_string(), "D".to_string(), "related_to".to_string(), 1.0, Some("mem3".to_string()), None, None);
        graph.add_relation("D".to_string(), "C".to_string(), "related_to".to_string(), 1.0, Some("mem4".to_string()), None, None);

        let pathfinder = Pathfinder::new(&graph);

        // Test shortest path
        let shortest = pathfinder.shortest_path("A", "C");
        assert_eq!(shortest.len(), 2);
        assert_eq!(shortest[0].source, "A");
        assert_eq!(shortest[0].target, "B");
        assert_eq!(shortest[1].source, "B");
        assert_eq!(shortest[1].target, "C");

        // Test k-hop expansion
        let expansion = pathfinder.k_hop_expansion("A", 1);
        assert_eq!(expansion.len(), 2); // A->B and A->D

        // Test all paths
        let all = pathfinder.all_paths("A", "C", 3);
        assert_eq!(all.len(), 2); // A->B->C and A->D->C
    }
}
