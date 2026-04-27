use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoredDocument {
    pub document: Document,
    pub score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoredVector {
    pub id: String,
    pub score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HybridScore {
    pub id: String,
    pub score: f32,
}

pub struct Bm25Index {
    documents: Vec<Document>,
    doc_term_freqs: Vec<HashMap<String, usize>>,
    doc_lengths: Vec<usize>,
    avg_dl: f32,
    doc_freqs: HashMap<String, usize>,
    k1: f32,
    b: f32,
}

impl Bm25Index {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            doc_term_freqs: Vec::new(),
            doc_lengths: Vec::new(),
            avg_dl: 0.0,
            doc_freqs: HashMap::new(),
            k1: 1.2,
            b: 0.75,
        }
    }

    pub fn index(&mut self, doc: Document) {
        let tokens = self.tokenize(&doc.content);
        let mut term_freqs = HashMap::new();
        for token in &tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        for term in term_freqs.keys() {
            *self.doc_freqs.entry(term.clone()).or_insert(0) += 1;
        }

        self.doc_lengths.push(tokens.len());
        self.doc_term_freqs.push(term_freqs);
        self.documents.push(doc);

        self.avg_dl = self.doc_lengths.iter().sum::<usize>() as f32 / self.documents.len() as f32;
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredDocument> {
        if self.documents.is_empty() {
            return Vec::new();
        }

        let query_tokens = self.tokenize(query);
        let mut scores = Vec::new();

        let n = self.documents.len() as f32;

        for (i, doc) in self.documents.iter().enumerate() {
            let mut score = 0.0;
            let dl = self.doc_lengths[i] as f32;
            let term_freqs = &self.doc_term_freqs[i];

            for token in &query_tokens {
                if let Some(&nq) = self.doc_freqs.get(token) {
                    let idf = ((n - nq as f32 + 0.5) / (nq as f32 + 0.5) + 1.0).ln();
                    let fq = term_freqs.get(token).cloned().unwrap_or(0) as f32;
                    let numerator = fq * (self.k1 + 1.0);
                    let denominator = fq + self.k1 * (1.0 - self.b + self.b * dl / self.avg_dl);
                    score += idf * numerator / denominator;
                }
            }

            if score > 0.0 {
                scores.push(ScoredDocument {
                    document: doc.clone(),
                    score,
                });
            }
        }

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        scores
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

// Búsqueda híbrida: BM25 + vector
pub fn hybrid_search(
    vector_results: Vec<ScoredVector>,
    bm25_results: Vec<ScoredDocument>,
    alpha: f32, // 0.0 = solo BM25, 1.0 = solo vector
) -> Vec<HybridScore> {
    let mut scores = HashMap::new();

    // Normalize BM25 scores if not empty
    let max_bm25 = bm25_results.iter().map(|r| r.score).fold(0.0, f32::max);

    for res in bm25_results {
        let normalized_score = if max_bm25 > 0.0 { res.score / max_bm25 } else { 0.0 };
        scores.insert(res.document.id.clone(), normalized_score * (1.0 - alpha));
    }

    // Normalize vector scores if not empty (assuming they are already somewhat normalized, but let's be safe)
    let max_vec = vector_results.iter().map(|r| r.score).fold(0.0, f32::max);

    for res in vector_results {
        let normalized_score = if max_vec > 0.0 { res.score / max_vec } else { 0.0 };
        let entry = scores.entry(res.id.clone()).or_insert(0.0);
        *entry += normalized_score * alpha;
    }

    let mut result: Vec<HybridScore> = scores
        .into_iter()
        .map(|(id, score)| HybridScore { id, score })
        .collect();

    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    result
}
