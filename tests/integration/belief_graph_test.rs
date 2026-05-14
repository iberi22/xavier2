//! Belief Graph Module Tests

#[cfg(test)]
mod belief_graph_tests {
    use xavier::memory::belief_graph::{Belief, Confidence, BeliefGraph};
    use xavier::domain::memory::belief::BeliefEdge;

    #[test]
    fn test_belief_creation() {
        let belief = Belief::new(
            "xavier".to_string(),
            "is_a".to_string(),
            "memory system".to_string(),
            Confidence::High,
        );

        assert_eq!(belief.subject, "xavier");
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
            0.8,
            "test_source".to_string(),
        );

        assert_eq!(edge.source, "node1");
        assert_eq!(edge.target, "node2");
        assert_eq!(edge.relation_type, "relates_to");
        assert_eq!(edge.confidence_score, 0.8);
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

        graph.add_belief(belief, None).await.unwrap();

        let nodes = graph.list_nodes();
        assert!(!nodes.is_empty());
    }

    #[tokio::test]
    async fn test_belief_graph_add_edge() {
        let graph = BeliefGraph::new();

        // Add two beliefs
        let belief1 = Belief::new(
            "xavier".to_string(),
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

        graph.add_belief(belief1, None).await.unwrap();
        graph.add_belief(belief2, None).await.unwrap();

        // Add relation manually
        graph
            .add_relation(
                "xavier".to_string(),
                "memory".to_string(),
                "includes".to_string(),
                Some("manual_provenance".to_string()),
                Some("user_input"),
            )
            .await
            .unwrap();

        let edges = graph.get_edges();
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
            )
            .await
            .unwrap();
        graph
            .add_belief(
                Belief::new(
                    "B".to_string(),
                    "links".to_string(),
                    "C".to_string(),
                    Confidence::High,
                ),
                None,
            )
            .await
            .unwrap();

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
                    "xavier".to_string(),
                    "is".to_string(),
                    "memory".to_string(),
                    Confidence::High,
                ),
                None,
            )
            .await
            .unwrap();
        graph
            .add_belief(
                Belief::new(
                    "xavier".to_string(),
                    "supports".to_string(),
                    "AI".to_string(),
                    Confidence::High,
                ),
                None,
            )
            .await
            .unwrap();
        graph
            .add_belief(
                Belief::new(
                    "trading".to_string(),
                    "uses".to_string(),
                    "xavier".to_string(),
                    Confidence::Medium,
                ),
                None,
            )
            .await
            .unwrap();

        // Search for xavier-related beliefs
        let results = graph.search("xavier").await;
        assert!(results.len() >= 2);
    }

    #[tokio::test]
    async fn test_belief_graph_contradiction() {
        let graph = BeliefGraph::new();

        // Add first belief
        graph
            .add_relation(
                "Rust".to_string(),
                "Memory Safety".to_string(),
                "provides".to_string(),
                Some("source1".to_string()),
                Some("verified_fact"),
            )
            .await
            .unwrap();

        // Add contradicting belief (same triple, different source)
        graph
            .add_relation(
                "Rust".to_string(),
                "Memory Safety".to_string(),
                "provides".to_string(),
                Some("source2".to_string()),
                Some("user_input"),
            )
            .await
            .unwrap();

        let edges = graph.get_edges();
        assert_eq!(edges.len(), 2);

        let edge2 = edges.iter().find(|e| e.provenance_id == "source2").unwrap();
        assert!(edge2.contradicts_edge_id.is_some());

        let edge1 = edges.iter().find(|e| e.provenance_id == "source1").unwrap();
        assert_eq!(edge2.contradicts_edge_id.as_ref().unwrap(), &edge1.id);
    }
}

#[cfg(test)]
mod belief_serialization_tests {
    use xavier::memory::belief_graph::{Belief, Confidence};

    #[test]
    fn test_belief_serialization() {
        let belief = Belief::new(
            "test".to_string(),
            "relates".to_string(),
            "example".to_string(),
            Confidence::High,
        );

        let json = serde_json::to_string(&belief).expect("test assertion");
        assert!(json.contains("test"));
        assert!(json.contains("example"));
    }

    #[test]
    fn test_belief_deserialization() {
        let json = r#"{
            "subject": "xavier",
            "predicate": "is_a",
            "object": "system",
            "confidence": "High"
        }"#;

        let belief: Belief = serde_json::from_str(json).expect("test assertion");
        assert_eq!(belief.subject, "xavier");
        assert_eq!(belief.confidence, Confidence::High);
    }
}
