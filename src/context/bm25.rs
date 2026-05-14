use std::collections::{HashMap, HashSet};

use super::ContextDocument;

const K1: f32 = 1.5;
const B: f32 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub struct Bm25Hit {
    pub document: ContextDocument,
    pub score: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Bm25Index {
    documents: Vec<ContextDocument>,
    document_frequency: HashMap<String, usize>,
    average_document_length: f32,
}

impl Bm25Index {
    pub fn new(documents: Vec<ContextDocument>) -> Self {
        let mut document_frequency = HashMap::new();
        let mut total_length = 0usize;

        for document in &documents {
            let tokens = tokenize(&document.content);
            total_length += tokens.len();

            let unique_terms: HashSet<_> = tokens.into_iter().collect();
            for term in unique_terms {
                *document_frequency.entry(term).or_insert(0) += 1;
            }
        }

        let average_document_length = if documents.is_empty() {
            0.0
        } else {
            total_length as f32 / documents.len() as f32
        };

        Self {
            documents,
            document_frequency,
            average_document_length,
        }
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Bm25Hit> {
        if limit == 0 || self.documents.is_empty() {
            return Vec::new();
        }

        let query_terms = tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let avg_len = self.average_document_length.max(1.0);
        let document_count = self.documents.len() as f32;
        let mut hits = Vec::new();

        for document in &self.documents {
            let doc_tokens = tokenize(&document.content);
            let doc_len = doc_tokens.len() as f32;
            let term_frequencies = term_frequencies(&doc_tokens);

            let mut score = 0.0;
            for term in &query_terms {
                let tf = *term_frequencies.get(term).unwrap_or(&0) as f32;
                if tf == 0.0 {
                    continue;
                }

                let df = *self.document_frequency.get(term).unwrap_or(&0) as f32;
                let idf = (((document_count - df + 0.5) / (df + 0.5)) + 1.0).ln();
                let numerator = tf * (K1 + 1.0);
                let denominator = tf + K1 * (1.0 - B + B * (doc_len / avg_len));
                score += idf * (numerator / denominator.max(f32::EPSILON));
            }

            if score > 0.0 {
                hits.push(Bm25Hit {
                    document: document.clone(),
                    score,
                });
            }
        }

        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.document.created_at.cmp(&left.document.created_at))
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
        hits.truncate(limit);
        hits
    }
}

pub fn tokenize(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|token| token.to_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn term_frequencies(tokens: &[String]) -> HashMap<String, usize> {
    let mut frequencies = HashMap::new();
    for token in tokens {
        *frequencies.entry(token.clone()).or_insert(0) += 1;
    }
    frequencies
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn doc(id: &str, content: &str, seconds: i64) -> ContextDocument {
        ContextDocument::new(id, "session-1", "user", content).with_created_at(
            Utc.timestamp_opt(seconds, 0)
                .single()
                .expect("test assertion"),
        )
    }

    #[test]
    fn tokenizes_whitespace_and_lowercases() {
        assert_eq!(
            tokenize("Rust  Async\tTOOLS"),
            vec!["rust", "async", "tools"]
        );
    }

    #[test]
    fn ranks_matching_document_first() {
        let index = Bm25Index::new(vec![
            doc("1", "rust async runtime orchestration", 1),
            doc("2", "python notebooks and pandas", 2),
            doc("3", "async rust task orchestration", 3),
        ]);

        let hits = index.search("rust orchestration", 10);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].document.id, "3");
        assert_eq!(hits[1].document.id, "1");
        assert!(hits[0].score >= hits[1].score);
    }

    #[test]
    fn empty_query_or_limit_returns_no_hits() {
        let index = Bm25Index::new(vec![doc("1", "hello world", 1)]);

        assert!(index.search("", 10).is_empty());
        assert!(index.search("hello", 0).is_empty());
    }
}
