//! App-layer SecurityService — delegates to the real `security::SecurityService`.
//!
//! This implements both `SecurityScanPort` and `InputSecurityPort` by wrapping the concrete `security::SecurityService`.
//! Handlers should use these through port traits, not call `security::SecurityService` directly.
/// NOTE: HexArch improvement — depends on concrete crate::security::SecurityService, should use a port abstraction
use crate::domain::security::{ScanResult, Severity as DomainSeverity, Threat as DomainThreat, ThreatCategory as DomainThreatCategory, ThreatLevel as DomainThreatLevel};
use crate::ports::inbound::security_port::SecureInputResult;
use crate::ports::inbound::{InputSecurityPort, SecurityScanPort};
use crate::ports::outbound::ThreatDetectionPort;
use crate::security::{self, Anticipator};
use crate::security::threat_store::SecurityThreatStore;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

/// Wrapper that delegates to the real `security::SecurityService`.
pub struct SecurityService {
    threat_store: Option<Arc<SecurityThreatStore>>,
    anticipator: Anticipator,
}

impl Default for SecurityService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityService {
    pub fn new() -> Self {
        Self {
            threat_store: None,
            anticipator: Anticipator::new(),
        }
    }

    pub fn with_store(store: Arc<SecurityThreatStore>) -> Self {
        Self {
            threat_store: Some(store),
            anticipator: Anticipator::new(),
        }
    }
}

#[async_trait]
impl SecurityScanPort for SecurityService {
    /// Scans the given target for security threats.
    /// Delegates to `security::SecurityService::process_input()`.
    async fn scan(&self, target: &str, _level: Option<DomainThreatLevel>) -> anyhow::Result<ScanResult> {
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
}

#[async_trait]
impl InputSecurityPort for SecurityService {
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

#[async_trait]
impl ThreatDetectionPort for SecurityService {
    async fn scan_and_log(&self, text: &str, component: &str) -> anyhow::Result<bool> {
        let result = self.anticipator.scan(text);

        if !result.clean {
            if let Some(ref store) = self.threat_store {
                for threat in &result.threats {
                    let _ = store.save_threat(threat, component);
                }
            }
        }

        Ok(result.clean)
    }
}

/// Converts a security `DetectionResult` into domain `Threat` entities.
fn detection_to_threats(detection: &security::ProcessResult, _target: &str) -> Vec<DomainThreat> {
    if !detection.detection.is_injection && detection.detection.confidence < 0.1 {
        return Vec::new();
    }

    let severity = match detection.detection.attack_type {
        security::AttackType::DirectPromptInjection => DomainSeverity::Critical,
        security::AttackType::IndirectPromptInjection => DomainSeverity::High,
        security::AttackType::PromptLeaking => DomainSeverity::Medium,
        security::AttackType::None => DomainSeverity::Low,
    };

    let threat_level = match severity {
        DomainSeverity::Critical => DomainThreatLevel::Critical,
        DomainSeverity::High => DomainThreatLevel::High,
        DomainSeverity::Medium => DomainThreatLevel::Medium,
        DomainSeverity::Low => DomainThreatLevel::Low,
    };

    let category = match detection.detection.attack_type {
        security::AttackType::DirectPromptInjection
        | security::AttackType::IndirectPromptInjection => DomainThreatCategory::Injection,
        security::AttackType::PromptLeaking => DomainThreatCategory::DataExposure,
        security::AttackType::None => DomainThreatCategory::Injection,
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

    vec![DomainThreat {
        id: ulid::Ulid::new().to_string(),
        name: name.to_string(),
        category,
        level: threat_level,
        severity,
        description,
        affected_component: "xavier".to_string(),
        remediation: Some(if detection.sanitized_input.is_some() {
            "Input was sanitized. Review sanitized version before processing.".to_string()
        } else {
            "Consider enabling auto-sanitization for this input type.".to_string()
        }),
        discovered_at: Utc::now(),
    }]
}
