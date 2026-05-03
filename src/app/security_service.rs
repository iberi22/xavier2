//! App-layer SecurityService — delegates to the real `security::SecurityService`.
//!
//! This implements both `SecurityScanPort` and `InputSecurityPort` by wrapping the concrete `security::SecurityService`.
//! Handlers should use these through port traits, not call `security::SecurityService` directly.

use crate::domain::security::{ScanResult, Severity, Threat, ThreatCategory, ThreatLevel};
use crate::ports::inbound::{SecurityScanPort, InputSecurityPort};
use crate::ports::inbound::security_port::SecureInputResult;
use crate::security;
use async_trait::async_trait;
use chrono::Utc;
use std::time::Instant;

/// Wrapper that delegates to the real `security::SecurityService`.
pub struct SecurityService;

impl Default for SecurityService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanPort for SecurityService {
    /// Scans the given target for security threats.
    /// Delegates to `security::SecurityService::process_input()`.
    async fn scan(&self, target: &str, _level: Option<ThreatLevel>) -> anyhow::Result<ScanResult> {
        let start = Instant::now();

        // The underlying service is sync; run it on the blocking thread pool.
        let detection = tokio::task::spawn_blocking({
            let target = target.to_string();
            move || {
                let service = security::get_security_service();
                service.process_input(&target)
            }
        })
        .await?;

        let scan_duration_ms = start.elapsed().as_millis() as u64;
        let threats = detection_to_threats(&detection, target);

        let scan_result = ScanResult {
            id: ulid::Ulid::new().to_string(),
            scanned_target: target.to_string(),
            threats,
            scan_duration_ms,
            completed_at: Utc::now(),
        };

        Ok(scan_result)
    }

    /// Returns the current security configuration as JSON.
    async fn get_config(&self) -> anyhow::Result<serde_json::Value> {
        let config = tokio::task::spawn_blocking(|| {
            let service = security::get_security_service();
            service.get_config()
        })
        .await?;

        Ok(serde_json::json!({
            "enabled": config.enabled,
            "encryption_algorithm": config.encryption_algorithm,
            "enable_direct_detection": config.enable_direct_detection,
            "enable_indirect_detection": config.enable_indirect_detection,
            "enable_leaking_detection": config.enable_leaking_detection,
            "min_confidence_threshold": config.min_confidence_threshold,
            "auto_sanitize": config.auto_sanitize,
            "filter_output": config.filter_output,
            "paranoid_mode": config.paranoid_mode,
        }))
    }

    /// Processes input and returns a secure result.
    async fn process_input(&self, input: &str) -> anyhow::Result<SecureInputResult> {
        let input = input.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let service = security::get_security_service();
            service.process_input(&input)
        })
        .await?;

        Ok(SecureInputResult {
            allowed: result.allowed,
            sanitized_input: result.sanitized_input,
            original_input: result.original_input,
            detection_confidence: result.detection.confidence,
            is_injection: result.detection.is_injection,
            attack_type: result.detection.attack_type.as_str().to_string(),
        })
    }
}

#[async_trait]
impl InputSecurityPort for SecurityService {
    async fn process_input(&self, input: &str) -> anyhow::Result<SecureInputResult> {
        // Reuse the implementation from SecurityScanPort
        SecurityScanPort::process_input(self, input).await
    }

    async fn process_output(&self, output: &str) -> anyhow::Result<String> {
        let output = output.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let service = security::get_security_service();
            service.process_output(&output)
        })
        .await?;
        Ok(result)
    }
}

/// Converts a security `DetectionResult` into domain `Threat` entities.
fn detection_to_threats(detection: &security::ProcessResult, _target: &str) -> Vec<Threat> {
    if !detection.detection.is_injection && detection.detection.confidence < 0.1 {
        return Vec::new();
    }

    let severity = match detection.detection.attack_type {
        security::AttackType::DirectPromptInjection => Severity::Critical,
        security::AttackType::IndirectPromptInjection => Severity::High,
        security::AttackType::PromptLeaking => Severity::Medium,
        security::AttackType::None => Severity::Low,
    };

    let threat_level = match severity {
        Severity::Critical => ThreatLevel::Critical,
        Severity::High => ThreatLevel::High,
        Severity::Medium => ThreatLevel::Medium,
        Severity::Low => ThreatLevel::Low,
    };

    let category = match detection.detection.attack_type {
        security::AttackType::DirectPromptInjection
        | security::AttackType::IndirectPromptInjection => ThreatCategory::Injection,
        security::AttackType::PromptLeaking => ThreatCategory::DataExposure,
        security::AttackType::None => ThreatCategory::Injection,
    };

    let name = match detection.detection.attack_type {
        security::AttackType::DirectPromptInjection => "Direct Prompt Injection",
        security::AttackType::IndirectPromptInjection => "Indirect Prompt Injection",
        security::AttackType::PromptLeaking => "Prompt Leaking",
        security::AttackType::None => "Low-Confidence Suspicious Input",
    };

    let description = format!(
        "{} (confidence: {:.2}). Message: {}",
        name, detection.detection.confidence, detection.detection.message
    );

    vec![Threat {
        id: ulid::Ulid::new().to_string(),
        name: name.to_string(),
        category,
        level: threat_level,
        severity,
        description,
        affected_component: "xavier2".to_string(),
        remediation: Some(if detection.sanitized_input.is_some() {
            "Input was sanitized. Review sanitized version before processing.".to_string()
        } else {
            "Consider enabling auto-sanitization for this input type.".to_string()
        }),
        discovered_at: Utc::now(),
    }]
}
