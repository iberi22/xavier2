use crate::context::ContextLevel;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CachedContext {
    pub content: String,
    pub level: ContextLevel,
    pub expires_at: Instant,
}

pub struct ContextManager {
    cache: HashMap<String, CachedContext>,
    ttl: Duration,
    max_size_chars: usize,
}

impl ContextManager {
    pub fn new(ttl_seconds: u64, max_size_chars: usize) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            max_size_chars,
        }
    }

    pub fn get(&mut self, session_id: &str) -> Option<String> {
        if let Some(entry) = self.cache.get(session_id) {
            if entry.expires_at < Instant::now() {
                self.cache.remove(session_id);
                return None;
            }
            return Some(entry.content.clone());
        }
        None
    }

    pub fn put(&mut self, session_id: &str, content: String, level: ContextLevel) {
        let sanitized_content = if content.len() > self.max_size_chars {
            content.chars().take(self.max_size_chars).collect()
        } else {
            content
        };

        self.cache.insert(
            session_id.to_string(),
            CachedContext {
                content: sanitized_content,
                level,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, v| v.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_caching_and_expiration() {
        let mut manager = ContextManager::new(1, 100);
        manager.put("s1", "some context".to_string(), ContextLevel::Medium);

        assert_eq!(manager.get("s1").unwrap(), "some context");

        // Wait for expiration
        std::thread::sleep(Duration::from_secs(2));
        assert!(manager.get("s1").is_none());
    }

    #[test]
    fn context_size_limit() {
        let mut manager = ContextManager::new(60, 5);
        manager.put("s1", "long context".to_string(), ContextLevel::Minimal);

        assert_eq!(manager.get("s1").unwrap(), "long ");
    }
}
