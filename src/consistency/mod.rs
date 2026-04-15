//! Consistency module - Memory coherence and retention regularization
//!
//! This module provides retention regularization for detecting conflicts,
//! verifying entity consistency, checking temporal ordering, and scoring
//! memory coherence across the multi-layer memory architecture.

pub mod regularization;

pub use regularization::{
    CoherenceReport, Conflict, ConflictType, DriftAlert, DriftThreshold, DriftType,
    RegularizerConfig, RetentionRegularizer, TemporalIssue,
};
