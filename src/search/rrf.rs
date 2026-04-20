use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ScoredResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
}

#[derive(Clone, Debug)]
struct FusedScore {
    id: String,
    content: String,
    best_original_score: f32,
    source: String,
    total_rrf: f32,
    total_weight: f32,
}

impl FusedScore {
    fn new(result: ScoredResult, contribution: f32, weight: f32) -> Self {
        let best_original_score = result.score;
        Self {
            id: result.id,
            content: result.content,
            best_original_score,
            source: result.source,
            total_rrf: contribution,
            total_weight: weight,
        }
    }

    fn add_score(&mut self, result: &ScoredResult, contribution: f32, weight: f32) {
        if result.score > self.best_original_score {
            self.best_original_score = result.score;
            self.content = result.content.clone();
            self.source = result.source.clone();
        }
        self.total_rrf += contribution;
        self.total_weight += weight;
    }

    fn into_result(self) -> ScoredResult {
        ScoredResult {
            id: self.id,
            content: self.content,
            score: self.total_rrf,
            source: "hybrid".to_string(),
        }
    }
}

/// Reciprocal Rank Fusion.
///
/// Result positions are treated as 1-based ranks.
pub fn reciprocal_rank_fusion(result_sets: Vec<Vec<ScoredResult>>, k: u32) -> Vec<ScoredResult> {
    let mut scores: HashMap<String, FusedScore> = HashMap::new();

    for result_set in result_sets {
        for (index, result) in result_set.into_iter().enumerate() {
            let rank = (index as u32) + 1;
            let contribution = 1.0 / ((k + rank) as f32);
            let weight = 1.0;

            scores
                .entry(result.id.clone())
                .and_modify(|entry| entry.add_score(&result, contribution, weight))
                .or_insert_with(|| FusedScore::new(result, contribution, weight));
        }
    }

    let mut ranked: Vec<_> = scores.into_values().collect();
    ranked.sort_by(|left, right| {
        right
            .total_rrf
            .partial_cmp(&left.total_rrf)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .total_weight
                    .partial_cmp(&left.total_weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    ranked.into_iter().map(FusedScore::into_result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_fusion_two_result_sets() {
        let results = vec![
            vec![
                ScoredResult {
                    id: "a".into(),
                    content: "alpha".into(),
                    score: 1.0,
                    source: "keyword".into(),
                },
                ScoredResult {
                    id: "b".into(),
                    content: "bravo".into(),
                    score: 0.9,
                    source: "keyword".into(),
                },
                ScoredResult {
                    id: "c".into(),
                    content: "charlie".into(),
                    score: 0.8,
                    source: "keyword".into(),
                },
            ],
            vec![
                ScoredResult {
                    id: "b".into(),
                    content: "bravo".into(),
                    score: 1.0,
                    source: "vector".into(),
                },
                ScoredResult {
                    id: "d".into(),
                    content: "delta".into(),
                    score: 0.9,
                    source: "vector".into(),
                },
                ScoredResult {
                    id: "a".into(),
                    content: "alpha".into(),
                    score: 0.8,
                    source: "vector".into(),
                },
            ],
        ];

        let fused = reciprocal_rank_fusion(results, 60);
        let ids: Vec<_> = fused.iter().map(|result| result.id.clone()).collect();

        assert_eq!(ids[0], "b");
        assert_eq!(ids[1], "a");
    }

    #[test]
    fn test_rrf_with_empty_result_set() {
        let results = vec![
            vec![ScoredResult {
                id: "a".into(),
                content: "alpha".into(),
                score: 1.0,
                source: "keyword".into(),
            }],
            vec![],
        ];

        let fused = reciprocal_rank_fusion(results, 60);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].id, "a");
    }
}
