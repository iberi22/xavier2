// ============================================
// Tests for Xavier2 Memory System
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Keyword Search Tests ====================

    #[test]
    fn test_keyword_extraction() {
        let content = "This is a test document about Next.js and Supabase";
        let keywords = extract_keywords(content);

        assert!(keywords.contains(&"nextjs".to_string()));
        assert!(keywords.contains(&"supabase".to_string()));
        assert!(!keywords.contains(&"this".to_string())); // stop word
    }

    #[test]
    fn test_search_returns_results() {
        // This test verifies that search actually returns results
        // after adding documents
        let mut index = SimpleMemoryIndex::new();

        // Add document
        let doc = SimpleMemoryDoc::new(
            "test.rs".to_string(),
            "fn main() { println!(\"Hello\"); }".to_string(),
            serde_json::json!({"type": "test"})
        );
        index.add(doc);

        // Search should return results
        let results = index.search("Hello", 5);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_keyword_scoring() {
        let mut index = SimpleMemoryIndex::new();

        // Add multiple docs
        index.add(SimpleMemoryDoc::new(
            "test1.rs".to_string(),
            "fn main() { println!(\"test\"); }".to_string(),
            serde_json::json!({})
        ));

        index.add(SimpleMemoryDoc::new(
            "test2.rs".to_string(),
            "other content".to_string(),
            serde_json::json!({})
        ));

        let results = index.search("test", 5);
        assert!(results.len() > 0);
        assert!(results[0].score > 0.0);
    }

    // ==================== Embedding Tests ====================

    #[test]
    fn test_embedding_generation() {
        // Test that we can call pplx-embed and get embeddings
        // This is a placeholder - actual test would make HTTP call
        let text = "Test document content";
        assert!(!text.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];

        // Same vectors should have similarity 1.0
        let sim = cosine_similarity(&vec1, &vec2);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0];

        // Orthogonal vectors should have similarity 0.0
        let sim = cosine_similarity(&vec1, &vec2);
        assert!(sim.abs() < 0.001);
    }

    // ==================== Checkpoint Tests ====================

    #[test]
    fn test_checkpoint_size_limit() {
        let checkpoint = Checkpoint::from_session(
            "Fixed authentication bug",
            vec!["auth.rs".to_string()],
            vec!["commit abc123".to_string()],
            vec!["Fix login".to_string()],
        );

        // Checkpoint should be under 2KB
        assert!(checkpoint.size() < 2048);
    }

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::new();

        assert!(!checkpoint.id.is_empty());
        assert!(checkpoint.timestamp > 0);
    }

    #[test]
    fn test_checkpoint_serialization() {
        let checkpoint = Checkpoint::from_session(
            "Summary of work done",
            vec!["file1.rs".to_string()],
            vec!["git commit".to_string()],
            vec!["task 1".to_string()],
        );

        // Should serialize and deserialize correctly
        let json = serde_json::to_string(&checkpoint).unwrap();
        let restored: Checkpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(checkpoint.id, restored.id);
    }

    // ==================== Token Reduction Tests ====================

    #[test]
    fn test_token_savings() {
        let original = "x".repeat(56000); // 56KB
        let entry = VirtualMemoryEntry::new(
            "test.txt".to_string(),
            original.clone(),
            serde_json::json!({}),
        );

        let savings = TokenSavings::calculate(&original, &entry);

        // Should save more than 90%
        assert!(savings.reduction_percent > 90.0);
    }

    #[test]
    fn test_summary_creation() {
        let content = "a".repeat(1000);
        let entry = VirtualMemoryEntry::new(
            "test.txt".to_string(),
            content,
            serde_json::json!({}),
        );

        // Summary should be shorter than original
        assert!(entry.summary.len() < 1000);
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_hybrid_search() {
        let mut index = SimpleMemoryIndex::new();

        // Add docs
        index.add(SimpleMemoryDoc::new(
            "nextjs.rs".to_string(),
            "Next.js with Supabase authentication".to_string(),
            serde_json::json!({})
        ));

        // Should find by keywords
        let results = index.search("Next.js Supabase", 5);
        assert!(!results.is_empty());
    }
}
