//! In-memory context indexer.
//!
//! Provides a simple document store with BM25-based search over stored
//! [`ContextDocument`]s. Acts as the primary entry point for the context
//! regeneration pipeline.

use std::collections::HashMap;

use super::{
    hybrid::{ContextSearchHit, HybridContextSearch},
    ContextDocument,
};

/// In-memory document index with add/remove/search operations.
#[derive(Debug, Clone)]
pub struct ContextIndexer {
    documents: HashMap<String, ContextDocument>,
    search: HybridContextSearch,
}

impl Default for ContextIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextIndexer {
    /// Creates a new empty indexer.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            search: HybridContextSearch::default(),
        }
    }

    /// Returns the number of indexed documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns `true` if the indexer contains no documents.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Adds a document to the index, replacing any existing document with the
    /// same [`ContextDocument::id`].
    pub fn index_document(&mut self, doc: ContextDocument) {
        self.documents.insert(doc.id.clone(), doc);
    }

    /// Removes the document with the given `id`, returning `true` if a document
    /// was actually removed.
    pub fn remove_document(&mut self, id: &str) -> bool {
        self.documents.remove(id).is_some()
    }

    /// Searches the index for documents matching `query` and returns the most
    /// relevant ones (up to `limit`, default 10).
    ///
    /// Internally uses hybrid BM25 + metadata search with Reciprocal Rank Fusion.
    pub fn search(&self, query: &str, limit: usize) -> Vec<ContextDocument> {
        let limit = if limit == 0 { 10 } else { limit };
        let docs: Vec<_> = self.documents.values().cloned().collect();
        let hits: Vec<ContextSearchHit> = self.search.search(&docs, query, limit);
        hits.into_iter().map(|hit| hit.document).collect()
    }

    /// Searches the index for documents matching `query`, returning full
    /// [`ContextSearchHit`] results (document + score + sources).
    pub fn search_with_scores(&self, query: &str, limit: usize) -> Vec<ContextSearchHit> {
        let limit = if limit == 0 { 10 } else { limit };
        let docs: Vec<_> = self.documents.values().cloned().collect();
        self.search.search(&docs, query, limit)
    }

    /// Returns a cloned vector of all indexed documents, sorted by
    /// [`ContextDocument::created_at`] descending (newest first).
    pub fn all_documents(&self) -> Vec<ContextDocument> {
        let mut docs: Vec<_> = self.documents.values().cloned().collect();
        docs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        docs
    }

    /// Returns the document with the given `id`, if present.
    pub fn get(&self, id: &str) -> Option<ContextDocument> {
        self.documents.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn doc(id: &str, content: &str, seconds: i64) -> ContextDocument {
        ContextDocument::new(id, "session-1", "user", content)
            .with_created_at(Utc.timestamp_opt(seconds, 0).unwrap())
    }

    #[test]
    fn new_indexer_is_empty() {
        let indexer = ContextIndexer::new();
        assert!(indexer.is_empty());
        assert_eq!(indexer.len(), 0);
    }

    #[test]
    fn index_document_inserts_and_retrievable() {
        let mut indexer = ContextIndexer::new();
        let d = doc("1", "rust async runtime", 1);
        indexer.index_document(d.clone());
        assert_eq!(indexer.len(), 1);
        assert_eq!(indexer.get("1"), Some(d));
    }

    #[test]
    fn index_document_replaces_existing_id() {
        let mut indexer = ContextIndexer::new();
        indexer.index_document(doc("1", "original content", 1));
        indexer.index_document(doc("1", "updated content", 2));
        assert_eq!(indexer.len(), 1);
        assert_eq!(indexer.get("1").unwrap().content, "updated content");
    }

    #[test]
    fn remove_document_returns_true_when_present() {
        let mut indexer = ContextIndexer::new();
        indexer.index_document(doc("1", "to be removed", 1));
        assert!(indexer.remove_document("1"));
        assert!(indexer.get("1").is_none());
        assert!(indexer.is_empty());
    }

    #[test]
    fn remove_document_returns_false_when_missing() {
        let mut indexer = ContextIndexer::new();
        assert!(!indexer.remove_document("nonexistent"));
    }

    #[test]
    fn search_returns_matching_documents() {
        let mut indexer = ContextIndexer::new();
        indexer.index_document(doc("1", "rust async runtime", 1));
        indexer.index_document(doc("2", "python pandas data", 2));
        indexer.index_document(doc("3", "async rust tasks", 3));

        let results = indexer.search("rust async", 10);
        assert_eq!(results.len(), 2);
        let ids: Vec<_> = results.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"1"));
        assert!(ids.contains(&"3"));
    }

    #[test]
    fn search_respects_limit() {
        let mut indexer = ContextIndexer::new();
        for i in 0..5 {
            indexer.index_document(doc(&format!("{i}"), "rust async runtime", i));
        }

        let results = indexer.search("rust", 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_with_scores_returns_sources() {
        let mut indexer = ContextIndexer::new();
        indexer.index_document(doc("1", "rust async runtime", 1));

        let hits = indexer.search_with_scores("rust async", 10);
        assert!(!hits.is_empty());
        assert!(!hits[0].sources.is_empty());
    }

    #[test]
    fn all_documents_sorted_newest_first() {
        let mut indexer = ContextIndexer::new();
        indexer.index_document(doc("1", "oldest", 1));
        indexer.index_document(doc("2", "newest", 3));
        indexer.index_document(doc("3", "middle", 2));

        let all = indexer.all_documents();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "3");
        assert_eq!(all[2].id, "1");
    }
}
