//! BM25 (Best Matching 25) ranking algorithm implementation.
//!
//! BM25 is a ranking function used by search engines to estimate the relevance
//! of documents to a given search query.

use crate::memory::qmd_memory::MemoryDocument;

/// BM25 parameters.
#[derive(Debug, Clone, Copy)]
pub struct Bm25Params {
    /// k1 controls term frequency saturation. Typical value is 1.2 to 2.0.
    pub k1: f32,
    /// b controls document length normalization. Typical value is 0.75.
    pub b: f32,
}

impl Default for Bm25Params {
    fn default() -> Self {
        Self { k1: 1.5, b: 0.75 }
    }
}

/// Computes the BM25 score for a set of documents and a query.
pub fn score_documents(
    query: &str,
    documents: &[MemoryDocument],
    params: Bm25Params,
) -> Vec<(f32, String)> {
    if query.is_empty() || documents.is_empty() {
        return Vec::new();
    }

    let query_terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if query_terms.is_empty() {
        return Vec::new();
    }

    let n = documents.len() as f32;
    let mut avg_dl = 0.0;
    let mut doc_term_freqs = Vec::with_capacity(documents.len());
    let mut doc_lengths = Vec::with_capacity(documents.len());

    for doc in documents {
        let content_lower = doc.content.to_lowercase();
        let tokens: Vec<&str> = content_lower.split_whitespace().collect();
        let doc_len = tokens.len() as f32;
        avg_dl += doc_len;
        doc_lengths.push(doc_len);

        let mut term_freqs = std::collections::HashMap::new();
        for token in tokens {
            *term_freqs.entry(token.to_string()).or_insert(0) += 1;
        }
        doc_term_freqs.push(term_freqs);
    }

    avg_dl /= n;

    let mut scores = Vec::with_capacity(documents.len());

    // Optimization: Calculate n_qi for each query term once
    let mut query_term_nqi = std::collections::HashMap::new();
    for term in &query_terms {
        let n_qi = documents
            .iter()
            .filter(|d| d.content.to_lowercase().contains(term))
            .count() as f32;
        query_term_nqi.insert(term, n_qi);
    }

    for (i, doc) in documents.iter().enumerate() {
        let mut score = 0.0;
        let doc_len = doc_lengths[i];
        let term_freqs = &doc_term_freqs[i];

        for term in &query_terms {
            let n_qi = *query_term_nqi.get(term).unwrap_or(&0.0);

            // IDF calculation
            let idf = ((n - n_qi + 0.5) / (n_qi + 0.5) + 1.0).ln();

            let f_qi = *term_freqs.get(term).unwrap_or(&0) as f32;

            // BM25 term score
            let tf_component = (f_qi * (params.k1 + 1.0))
                / (f_qi + params.k1 * (1.0 - params.b + params.b * (doc_len / avg_dl)));

            score += idf * tf_component;
        }

        if score > 0.0 {
            scores.push((score, doc.id.clone().unwrap_or_else(|| doc.path.clone())));
        }
    }

    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mock_doc(id: &str, content: &str) -> MemoryDocument {
        MemoryDocument {
            id: Some(id.to_string()),
            path: format!("path/{}", id),
            content: content.to_string(),
            metadata: json!({}),
            content_vector: None,
            embedding: Vec::new(),
            ..Default::default()
        }
    }

    #[test]
    fn test_bm25_basic() {
        let docs = vec![
            mock_doc("1", "the quick brown fox"),
            mock_doc("2", "the lazy dog"),
            mock_doc("3", "the quick dog"),
        ];

        let results = score_documents("quick", &docs, Bm25Params::default());
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1, "3");
        assert_eq!(results[1].1, "1");
    }
}
