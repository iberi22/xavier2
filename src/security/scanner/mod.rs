//! Scanner module - Multi-layer security scanning
//!
//! This module provides multi-layer security scanning for prompt injection detection.

pub mod entropy;
pub mod phrase_matcher;
pub mod scanner_impl;

pub use entropy::{
    EntropyCalculator, EntropyRegion, EntropyScanner, EntropyThreshold, SecretDetector, SecretMatch,
};
pub use phrase_matcher::{PhraseMatch, PhraseMatcher, INJECTION_PATTERNS};
pub use scanner_impl::{
    is_threat, scan_text, DetectionLayer, ScanResult, ScannerConfig, SecurityScanner, ThreatLevel,
    TriggeredDetection, SCANNER,
};
