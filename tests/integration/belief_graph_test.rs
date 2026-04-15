//! Belief Graph Module Tests

#[cfg(test)]
mod belief_graph_tests {
    use xavier2::memory::belief_graph::{Belief, BeliefEdge, BeliefGraph, Confidence};

    #[test]
    fn test_belief_creation() {
        let belief = Belief::new(
            "xavier2".to_string(),
            "is_a".to_string(),
            "memory system".to_string(),
            Confidence::High,
        );

        assert_eq!(belief.subject, "xavier2");
        assert_eq!(belief.predicate, "is_a");
        assert_eq!(belief.object, "memory system");
    }

    #[test]
    fn test_belief_confidence_levels() {
        let high = Confidence::High;
        let medium = Confidence::Medium;
        let low = Confidence::Low;

        assert!(matches!(high, Confidence::High));
        assert!(matches!(medium, Confidence::Medium));
        assert!(matches!(low, Confidence::Low));
    }

    #[test]
    fn test_belief_edge_creation() {
        let edge = BeliefEdge::new(
            "node1".to_string(),
            "node2".to_string(),
            "relates_to".to_string(),
        );

        assert_eq!(edge.from, "node1");
        assert_eq!(edge.to, "node2");
        assert_eq!(edge.relation, "relates_to");
    }

    #[tokio::test]
    async fn test_belief_graph_add_node() {
        let graph = BeliefGraph::new();

        let belief = Belief::new(
            "AI".to_string(),
            "is_a".to_string(),
            "technology".to_string(),
            Confidence::High,
        );

        graph.add_belief(belief, None, None).await;

        let nodes = graph.get_nodes().await;
        assert!(!nodes.is_empty());
    }

    #[tokio::test]
    async fn test_belief_graph_add_edge() {
        let graph = BeliefGraph::new();

        // Add two beliefs
        let belief1 = Belief::new(
            "xavier2".to_string(),
            "is".to_string(),
            "memory".to_string(),
            Confidence::High,
        );
        let belief2 = Belief::new(
            "memory".to_string(),
            "is".to_string(),
            "storage".to_string(),
            Confidence::Medium,
        );

        graph.add_belief(belief1, None, None).await;
        graph.add_belief(belief2, None, None).await;

        // Add edge between them
        graph
            .add_edge(
                "xavier2".to_string(),
                "memory".to_string(),
                "includes".to_string(),
            )
            .await;

        let edges = graph.get_edges().await;
        assert!(!edges.is_empty());
    }

    #[tokio::test]
    async fn test_belief_graph_traversal() {
        let graph = BeliefGraph::new();

        // Create a simple graph: A -> B -> C
        graph
            .add_belief(
                Belief::new(
                    "A".to_string(),
                    "links".to_string(),
                    "B".to_string(),
                    Confidence::High,
                ),
                None,
                None,
            )
            .await;
        graph
            .add_belief(
                Belief::new(
                    "B".to_string(),
                    "links".to_string(),
                    "C".to_string(),
                    Confidence::High,
                ),
                None,
                None,
            )
            .await;

        // BFS from A should find B and C
        let reachable = graph.bfs("A").await;
        assert!(reachable.contains(&"B".to_string()));
        assert!(reachable.contains(&"C".to_string()));
    }

    #[tokio::test]
    async fn test_belief_graph_search() {
        let graph = BeliefGraph::new();

        // Add multiple beliefs
        graph
            .add_belief(
                Belief::new(
                    "xavier2".to_string(),
                    "is".to_string(),
                    "memory".to_string(),
                    Confidence::High,
                ),
                None,
                None,
            )
            .await;
        graph
            .add_belief(
                Belief::new(
                    "xavier2".to_string(),
                    "supports".to_string(),
                    "AI".to_string(),
                    Confidence::High,
                ),
                None,
                None,
            )
            .await;
        graph
            .add_belief(
                Belief::new(
                    "trading".to_string(),
                    "uses".to_string(),
                    "xavier2".to_string(),
                    Confidence::Medium,
                ),
                None,
                None,
            )
            .await;

        // Search for xavier2-related beliefs
        let results = graph.search("xavier2").await;
        assert!(results.len() >= 2);
    }
}

#[cfg(test)]
mod belief_serialization_tests {
    use xavier2::memory::belief_graph::{Belief, Confidence};

    #[test]
    fn test_belief_serialization() {
        let belief = Belief::new(
            "test".to_string(),
            "relates".to_string(),
            "example".to_string(),
            Confidence::High,
        );

        let json = serde_json::to_string(&belief).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("example"));
    }

    #[test]
    fn test_belief_deserialization() {
        let json = r#"{
            "subject": "xavier2",
            "predicate": "is_a",
            "object": "system",
            "confidence": "High"
        }"#;

        let belief: Belief = serde_json::from_str(json).unwrap();
        assert_eq!(belief.subject, "xavier2");
        assert_eq!(belief.confidence, Confidence::High);
    }
}
