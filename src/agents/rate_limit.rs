use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use parking_lot::RwLock;
use std::sync::Arc;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotaUsage {
    pub used: u64,
    pub limit: u64,
}

impl QuotaUsage {
    pub fn percentage(&self) -> f64 {
        if self.limit == 0 {
            0.0
        } else {
            (self.used as f64 / self.limit as f64) * 100.0
        }
    }

    pub fn remaining_percentage(&self) -> f64 {
        100.0 - self.percentage()
    }

    pub fn is_below_safety_threshold(&self, threshold_percent: f64) -> bool {
        self.remaining_percentage() < threshold_percent
    }
}

pub struct RateLimitManager {
    quotas: RwLock<HashMap<String, QuotaUsage>>,
}

impl Default for RateLimitManager {
    fn default() -> Self {
        let mut quotas = HashMap::new();
        // Default quotas for some providers
        quotas.insert("OpenCode Go".to_string(), QuotaUsage { used: 0, limit: 100000 });
        quotas.insert("DeepSeek".to_string(), QuotaUsage { used: 0, limit: 100000 });
        quotas.insert("MiniMax".to_string(), QuotaUsage { used: 0, limit: 100000 });

        Self {
            quotas: RwLock::new(quotas),
        }
    }
}

impl RateLimitManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_quotas(&self) -> HashMap<String, QuotaUsage> {
        self.quotas.read().clone()
    }

    pub fn update_usage(&self, provider: &str, used: u64) {
        let mut quotas = self.quotas.write();
        if let Some(usage) = quotas.get_mut(provider) {
            usage.used = used;
        }
    }

    pub fn add_usage(&self, provider: &str, delta: u64) {
        let mut quotas = self.quotas.write();
        if let Some(usage) = quotas.get_mut(provider) {
            usage.used += delta;
        }
    }

    pub fn set_limit(&self, provider: &str, limit: u64) {
        let mut quotas = self.quotas.write();
        quotas.entry(provider.to_string()).or_default().limit = limit;
    }

    pub fn is_below_safety_threshold(&self, provider: &str, threshold_percent: f64) -> bool {
        let quotas = self.quotas.read();
        if let Some(usage) = quotas.get(provider) {
            usage.is_below_safety_threshold(threshold_percent)
        } else {
            false
        }
    }
}

pub static GLOBAL_RATE_LIMITER: Lazy<Arc<RateLimitManager>> = Lazy::new(|| Arc::new(RateLimitManager::new()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_percentage() {
        let usage = QuotaUsage { used: 50, limit: 100 };
        assert_eq!(usage.percentage(), 50.0);
        assert_eq!(usage.remaining_percentage(), 50.0);
        assert!(!usage.is_below_safety_threshold(10.0));

        let low_usage = QuotaUsage { used: 95, limit: 100 };
        assert_eq!(low_usage.percentage(), 95.0);
        assert_eq!(low_usage.remaining_percentage(), 5.0);
        assert!(low_usage.is_below_safety_threshold(10.0));
    }

    #[test]
    fn test_rate_limit_manager() {
        let manager = RateLimitManager::new();
        manager.set_limit("test", 100);
        manager.update_usage("test", 50);

        let quotas = manager.get_quotas();
        assert_eq!(quotas.get("test").unwrap().used, 50);
        assert!(!manager.is_below_safety_threshold("test", 10.0));

        manager.add_usage("test", 45);
        assert!(manager.is_below_safety_threshold("test", 10.0));
    }
}
