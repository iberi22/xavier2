use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info};

#[derive(Debug, Default)]
pub struct ContextMetrics {
    pub total_requests: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
    pub total_tokens_used: AtomicUsize,
}

impl ContextMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, tokens: usize, cache_hit: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_tokens_used.fetch_add(tokens, Ordering::Relaxed);
        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        debug!(
            "Context request recorded: tokens={}, cache_hit={}",
            tokens, cache_hit
        );
    }

    pub fn get_hit_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);

        if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn report(&self) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let tokens = self.total_tokens_used.load(Ordering::Relaxed);

        info!("--- Context Monitoring Report ---");
        info!("Total Requests: {}", total);
        info!("Cache Hits: {} ({:.2}%)", hits, self.get_hit_rate());
        info!("Total Tokens Used: {}", tokens);
        info!("---------------------------------");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_report_metrics() {
        let metrics = ContextMetrics::new();
        metrics.record_request(100, true);
        metrics.record_request(200, false);
        metrics.record_request(150, true);

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.cache_hits.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.cache_misses.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.total_tokens_used.load(Ordering::Relaxed), 450);
        assert!(metrics.get_hit_rate() > 66.0);
    }
}
