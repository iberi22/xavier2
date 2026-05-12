//! Episodic Memory Layer - Session-based memory with summarization
//!
//! Implements the second layer of the Multi-Layer Memory Architecture.
//! Provides session-based grouping with:
//! - Session-based grouping of memory items
//! - Summary generation placeholder (LLM-powered)
//! - Key event extraction
//! - Sentiment tracking per session

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A key event extracted from a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Unique identifier for this event
    pub id: String,
    /// Brief description of the event
    pub description: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Importance score (0-1)
    pub importance: f32,
    /// Optional event type
    pub event_type: Option<String>,
}

/// A session summary with key events and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Unique session identifier
    pub session_id: String,
    /// When the session started
    pub start_time: DateTime<Utc>,
    /// When the session ended
    pub end_time: Option<DateTime<Utc>>,
    /// LLM-generated summary of the session
    pub summary: Option<String>,
    /// Key events extracted from the session
    pub key_events: Vec<KeyEvent>,
    /// Sentiment timeline (0-1 scale, one per turn/segment)
    pub sentiment_timeline: Vec<f32>,
    /// Number of turns/segments in this session
    pub turn_count: usize,
    /// Items aggregated into this session
    pub item_ids: Vec<String>,
    /// Whether summary has been generated
    pub is_summarized: bool,
}

impl SessionSummary {
    /// Create a new session summary
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            start_time: Utc::now(),
            end_time: None,
            summary: None,
            key_events: Vec::new(),
            sentiment_timeline: Vec::new(),
            turn_count: 0,
            item_ids: Vec::new(),
            is_summarized: false,
        }
    }

    /// Mark session as ended
    pub fn end(&mut self) {
        self.end_time = Some(Utc::now());
    }

    /// Add a key event
    pub fn add_event(&mut self, event: KeyEvent) {
        self.key_events.push(event);
    }

    /// Add sentiment score for a segment
    pub fn add_sentiment(&mut self, score: f32) {
        // Clamp sentiment to [0, 1]
        self.sentiment_timeline.push(score.clamp(0.0, 1.0));
    }

    /// Add an item ID to this session
    pub fn add_item(&mut self, item_id: impl Into<String>) {
        self.item_ids.push(item_id.into());
    }

    /// Increment turn count
    pub fn increment_turns(&mut self) {
        self.turn_count += 1;
    }

    /// Calculate average sentiment
    pub fn average_sentiment(&self) -> Option<f32> {
        if self.sentiment_timeline.is_empty() {
            None
        } else {
            Some(self.sentiment_timeline.iter().sum::<f32>() / self.sentiment_timeline.len() as f32)
        }
    }

    /// Mark as summarized (placeholder for LLM generation)
    pub fn mark_summarized(&mut self, summary: impl Into<String>) {
        self.summary = Some(summary.into());
        self.is_summarized = true;
    }
}

/// Episodic Memory configuration
#[derive(Debug, Clone)]
pub struct EpisodicMemoryConfig {
    /// Number of turns before generating a summary
    pub summary_window: usize,
    /// Maximum number of sessions to retain
    pub max_sessions: usize,
    /// Minimum importance score for key events
    pub min_event_importance: f32,
}

impl Default for EpisodicMemoryConfig {
    fn default() -> Self {
        Self {
            summary_window: 10,
            max_sessions: 50,
            min_event_importance: 0.5,
        }
    }
}

/// Episodic Memory - Session-based memory storage
///
/// # Overview
/// Episodic memory stores memories organized by sessions/contexts.
/// Each session has a summary, key events, and sentiment tracking.
///
/// # Example
/// ```rust
/// use xavier::memory::episodic::{EpisodicMemory, SessionSummary, KeyEvent};
/// use chrono::Utc;
///
/// let mut em = EpisodicMemory::new();
/// let mut session = SessionSummary::new("session-1");
/// session.add_event(KeyEvent {
///     id: "event-1".into(),
///     description: "User asked about pricing".into(),
///     timestamp: Utc::now(),
///     importance: 0.8,
///     event_type: Some("question".into()),
/// });
///
/// em.add_session(session);
/// assert_eq!(em.len(), 1);
/// ```
pub struct EpisodicMemory {
    config: EpisodicMemoryConfig,
    /// Session summaries indexed by session ID
    sessions: HashMap<String, SessionSummary>,
    /// Ordered list of session IDs (for LRU-style eviction)
    session_order: Vec<String>,
}

impl EpisodicMemory {
    /// Create new episodic memory with default config
    pub fn new() -> Self {
        Self::with_config(EpisodicMemoryConfig::default())
    }

    /// Create new episodic memory with custom config
    pub fn with_config(config: EpisodicMemoryConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            session_order: Vec::new(),
        }
    }

    /// Get current number of sessions
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&SessionSummary> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut SessionSummary> {
        self.sessions.get_mut(session_id)
    }

    /// Add a new session, evicting old ones if at capacity
    ///
    /// Returns the evicted session ID if any.
    pub fn add_session(&mut self, session: SessionSummary) -> Option<String> {
        // Evict oldest sessions if at capacity
        while self.sessions.len() >= self.config.max_sessions && !self.session_order.is_empty() {
            self.evict_oldest_session();
        }

        let id = session.session_id.clone();
        self.session_order.push(id.clone());
        self.sessions.insert(id, session);

        None
    }

    /// Add an item to an existing session
    pub fn add_to_session(&mut self, session_id: &str, item_id: impl Into<String>) -> Result<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.add_item(item_id);
        session.increment_turns();
        Ok(())
    }

    /// Add a key event to an existing session
    pub fn add_event_to_session(&mut self, session_id: &str, event: KeyEvent) -> Result<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Only add if above importance threshold
        if event.importance >= self.config.min_event_importance {
            session.add_event(event);
        }
        Ok(())
    }

    /// Add sentiment score to an existing session
    pub fn add_sentiment_to_session(&mut self, session_id: &str, score: f32) -> Result<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.add_sentiment(score);
        Ok(())
    }

    /// Mark a session as summarized (placeholder for LLM)
    ///
    /// In production, this would call an LLM to generate the summary.
    pub fn summarize_session(
        &mut self,
        session_id: &str,
        summary: impl Into<String>,
    ) -> Result<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.mark_summarized(summary);
        Ok(())
    }

    /// End a session
    pub fn end_session(&mut self, session_id: &str) -> Result<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.end();
        Ok(())
    }

    /// Evict the oldest session
    fn evict_oldest_session(&mut self) -> Option<SessionSummary> {
        if let Some(id) = self.session_order.first().cloned() {
            self.session_order.remove(0);
            self.sessions.remove(&id)
        } else {
            None
        }
    }

    /// Remove a session by ID
    pub fn remove_session(&mut self, session_id: &str) -> Option<SessionSummary> {
        if let Some(pos) = self.session_order.iter().position(|id| *id == session_id) {
            self.session_order.remove(pos);
        }
        self.sessions.remove(session_id)
    }

    /// Get all sessions ordered by recency (most recent first)
    pub fn recent_sessions(&self, limit: usize) -> Vec<&SessionSummary> {
        self.session_order
            .iter()
            .rev()
            .filter_map(|id| self.sessions.get(id))
            .take(limit)
            .collect()
    }

    /// Get all sessions that have summaries
    pub fn summarized_sessions(&self) -> Vec<&SessionSummary> {
        self.sessions.values().filter(|s| s.is_summarized).collect()
    }

    /// Get sessions with key events
    pub fn sessions_with_events(&self) -> Vec<&SessionSummary> {
        self.sessions
            .values()
            .filter(|s| !s.key_events.is_empty())
            .collect()
    }

    /// Search sessions by summary content (simple substring match)
    ///
    /// Returns sessions whose summary contains the query.
    pub fn search_sessions(&self, query: &str, limit: usize) -> Vec<&SessionSummary> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();

        self.sessions
            .values()
            .filter(|session| {
                // Check summary
                if let Some(ref summary) = session.summary {
                    if summary.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                // Check key events
                for event in &session.key_events {
                    if event.description.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                false
            })
            .take(limit)
            .collect()
    }

    /// Clear all sessions
    pub fn clear(&mut self) {
        self.sessions.clear();
        self.session_order.clear();
    }

    /// Get statistics about episodic memory
    pub fn stats(&self) -> EpisodicMemoryStats {
        let total_events: usize = self.sessions.values().map(|s| s.key_events.len()).sum();
        let summarized_count = self.sessions.values().filter(|s| s.is_summarized).count();
        let total_sentiment_scores: usize = self
            .sessions
            .values()
            .map(|s| s.sentiment_timeline.len())
            .sum();

        EpisodicMemoryStats {
            session_count: self.sessions.len(),
            max_sessions: self.config.max_sessions,
            total_key_events: total_events,
            summarized_sessions: summarized_count,
            total_sentiment_scores,
            average_turns_per_session: if self.sessions.is_empty() {
                0.0
            } else {
                self.sessions.values().map(|s| s.turn_count).sum::<usize>() as f32
                    / self.sessions.len() as f32
            },
        }
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Episodic memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryStats {
    pub session_count: usize,
    pub max_sessions: usize,
    pub total_key_events: usize,
    pub summarized_sessions: usize,
    pub total_sentiment_scores: usize,
    pub average_turns_per_session: f32,
}

/// Generate a summary for a session (placeholder for LLM integration)
///
/// This is a placeholder function. In production, this would:
/// 1. Collect all items in the session
/// 2. Send them to an LLM (e.g., GPT-4o-mini)
/// 3. Parse and return the generated summary
///
/// # Arguments
/// * `items` - The memory items in the session
/// * `events` - Key events from the session
///
/// # Returns
/// A generated summary string
pub async fn generate_session_summary(_items: &[String], _events: &[KeyEvent]) -> Result<String> {
    // Placeholder: In production, call LLM here
    // For now, return a simple placeholder
    Ok("Session summary placeholder - implement LLM integration".to_string())
}

/// Extract key events from session items (placeholder)
///
/// This is a placeholder function. In production, this would:
/// 1. Analyze session items for significant moments
/// 2. Rate their importance
/// 3. Return key events
///
/// # Arguments
/// * `items` - The memory items to analyze
/// * `min_importance` - Minimum importance threshold (0-1)
///
/// # Returns
/// A vector of key events
pub async fn extract_key_events(_items: &[String], _min_importance: f32) -> Result<Vec<KeyEvent>> {
    // Placeholder: In production, implement event extraction logic
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = SessionSummary::new("test-session");
        assert_eq!(session.session_id, "test-session");
        assert!(session.summary.is_none());
        assert!(session.key_events.is_empty());
        assert!(!session.is_summarized);
    }

    #[test]
    fn test_add_event() {
        let mut session = SessionSummary::new("test");
        let event = KeyEvent {
            id: "e1".into(),
            description: "Important moment".into(),
            timestamp: Utc::now(),
            importance: 0.8,
            event_type: Some("milestone".into()),
        };

        session.add_event(event.clone());
        assert_eq!(session.key_events.len(), 1);
        assert_eq!(session.key_events[0].description, "Important moment");
    }

    #[test]
    fn test_sentiment_tracking() {
        let mut session = SessionSummary::new("test");
        session.add_sentiment(0.8);
        session.add_sentiment(0.6);
        session.add_sentiment(1.0);
        session.add_sentiment(0.2); // Should be clamped to 0.0

        assert_eq!(session.sentiment_timeline.len(), 4);
        assert!((session.average_sentiment().expect("test assertion") - 0.65).abs() < 0.01);
    }

    #[test]
    fn test_sentiment_clamping() {
        let mut session = SessionSummary::new("test");
        session.add_sentiment(1.5); // Should clamp to 1.0
        session.add_sentiment(-0.5); // Should clamp to 0.0

        assert_eq!(session.sentiment_timeline[0], 1.0);
        assert_eq!(session.sentiment_timeline[1], 0.0);
    }

    #[test]
    fn test_mark_summarized() {
        let mut session = SessionSummary::new("test");
        assert!(!session.is_summarized);

        session.mark_summarized("This is a summary");
        assert!(session.is_summarized);
        assert_eq!(session.summary.as_ref().expect("test assertion"), "This is a summary");
    }

    #[test]
    fn test_add_to_episodic_memory() {
        let mut em = EpisodicMemory::new();
        let session = SessionSummary::new("session-1");

        em.add_session(session);
        assert_eq!(em.len(), 1);
        assert!(em.get_session("session-1").is_some());
    }

    #[test]
    fn test_session_eviction() {
        let mut em = EpisodicMemory::with_config(EpisodicMemoryConfig {
            summary_window: 10,
            max_sessions: 2,
            min_event_importance: 0.5,
        });

        em.add_session(SessionSummary::new("session-1"));
        em.add_session(SessionSummary::new("session-2"));

        // Adding third session should evict oldest
        em.add_session(SessionSummary::new("session-3"));

        assert_eq!(em.len(), 2);
        assert!(em.get_session("session-1").is_none());
        assert!(em.get_session("session-2").is_some());
        assert!(em.get_session("session-3").is_some());
    }

    #[test]
    fn test_add_item_to_session() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));

        em.add_to_session("session-1", "item-1").expect("test assertion");
        em.add_to_session("session-1", "item-2").expect("test assertion");

        let session = em.get_session("session-1").expect("test assertion");
        assert_eq!(session.item_ids.len(), 2);
        assert_eq!(session.turn_count, 2);
    }

    #[test]
    fn test_add_event_to_session() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));

        let event = KeyEvent {
            id: "e1".into(),
            description: "Test event".into(),
            timestamp: Utc::now(),
            importance: 0.3,
            event_type: None,
        };

        // Event below threshold should not be added
        em.add_event_to_session("session-1", event.clone()).expect("test assertion");
        assert_eq!(em.get_session("session-1").expect("test assertion").key_events.len(), 0);

        // Event above threshold should be added
        let high_importance_event = KeyEvent {
            id: "e2".into(),
            description: "Important event".into(),
            timestamp: Utc::now(),
            importance: 0.8,
            event_type: None,
        };
        em.add_event_to_session("session-1", high_importance_event)
            .expect("test assertion");
        assert_eq!(em.get_session("session-1").expect("test assertion").key_events.len(), 1);
    }

    #[test]
    fn test_recent_sessions() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));
        em.add_session(SessionSummary::new("session-2"));
        em.add_session(SessionSummary::new("session-3"));

        let recent = em.recent_sessions(2);
        assert_eq!(recent.len(), 2);
        // Most recent first
        assert_eq!(recent[0].session_id, "session-3");
        assert_eq!(recent[1].session_id, "session-2");
    }

    #[test]
    fn test_search_sessions() {
        let mut em = EpisodicMemory::new();

        let mut session1 = SessionSummary::new("session-1");
        session1.mark_summarized("User asked about pricing for ManteniApp");
        em.add_session(session1);

        let mut session2 = SessionSummary::new("session-2");
        session2.mark_summarized("Discussion about deployment options");
        em.add_session(session2);

        let results = em.search_sessions("pricing", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "session-1");

        let results2 = em.search_sessions("deployment", 10);
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].session_id, "session-2");

        let results3 = em.search_sessions("ManteniApp", 10);
        assert_eq!(results3.len(), 1);
    }

    #[test]
    fn test_summarized_sessions_filter() {
        let mut em = EpisodicMemory::new();

        let mut session1 = SessionSummary::new("session-1");
        session1.mark_summarized("Summary 1");
        em.add_session(session1);

        let session2 = SessionSummary::new("session-2");
        em.add_session(session2);

        let summarized = em.summarized_sessions();
        assert_eq!(summarized.len(), 1);
        assert_eq!(summarized[0].session_id, "session-1");
    }

    #[test]
    fn test_stats() {
        let mut em = EpisodicMemory::new();

        let mut session1 = SessionSummary::new("session-1");
        session1.mark_summarized("Summary");
        session1.add_event(KeyEvent {
            id: "e1".into(),
            description: "Event 1".into(),
            timestamp: Utc::now(),
            importance: 0.8,
            event_type: None,
        });
        em.add_session(session1);

        let stats = em.stats();
        assert_eq!(stats.session_count, 1);
        assert_eq!(stats.max_sessions, 50);
        assert_eq!(stats.total_key_events, 1);
        assert_eq!(stats.summarized_sessions, 1);
    }

    #[test]
    fn test_clear() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));
        em.add_session(SessionSummary::new("session-2"));

        em.clear();
        assert!(em.is_empty());
        assert_eq!(em.len(), 0);
    }

    #[test]
    fn test_remove_session() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));
        em.add_session(SessionSummary::new("session-2"));

        let removed = em.remove_session("session-1");
        assert!(removed.is_some());
        assert_eq!(removed.expect("test assertion").session_id, "session-1");
        assert_eq!(em.len(), 1);
        assert!(em.get_session("session-1").is_none());
    }

    #[test]
    fn test_end_session() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));

        em.end_session("session-1").expect("test assertion");
        assert!(em.get_session("session-1").expect("test assertion").end_time.is_some());
    }

    #[test]
    fn test_error_on_nonexistent_session() {
        let mut em = EpisodicMemory::new();
        em.add_session(SessionSummary::new("session-1"));

        let result = em.add_to_session("nonexistent", "item");
        assert!(result.is_err());

        let result = em.add_event_to_session(
            "nonexistent",
            KeyEvent {
                id: "e1".into(),
                description: "Test".into(),
                timestamp: Utc::now(),
                importance: 0.5,
                event_type: None,
            },
        );
        assert!(result.is_err());

        let result = em.add_sentiment_to_session("nonexistent", 0.5);
        assert!(result.is_err());
    }
}
