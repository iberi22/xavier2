//! Retrieval module - Multi-layer memory retrieval with adaptive gating
//!
//! This module provides adaptive retrieval gating that combines results from
//! Working, Episodic, and Semantic memory layers using weighted RRF fusion.

pub mod config;
pub mod gating;

pub use gating::{
    AdaptiveGating, Event, GatingConfig, LayerSearchResult, LayerStats, LayerWeights,
    SessionSummary,
};
