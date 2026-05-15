use anyhow::{anyhow, Result};

use xavier::ports::inbound::input_security_port::SecureInputResult;
use xavier::ports::inbound::InputSecurityPort;
use xavier::security::SecurityService;

pub fn blocked_external_input_response(
    label: &str,
    result: &SecureInputResult,
) -> serde_json::Value {
    serde_json::json!({
        "status": "blocked",
        "blocked": true,
        "reason": "security_policy_violation",
        "message": format!("{label} blocked by security policy"),
        "detection": {
            "is_injection": result.is_injection,
            "confidence": result.detection_confidence,
            "attack_type": result.attack_type,
        }
    })
}

pub async fn secure_external_input(
    security: &dyn InputSecurityPort,
    label: &str,
    input: &str,
) -> std::result::Result<String, serde_json::Value> {
    let result = security
        .process_input(input)
        .await
        .unwrap_or_else(|_| SecureInputResult {
            allowed: false,
            sanitized_input: None,
            original_input: input.to_string(),
            detection_confidence: 1.0,
            is_injection: true,
            attack_type: "unknown".to_string(),
        });

    if !result.allowed {
        return Err(blocked_external_input_response(label, &result));
    }

    Ok(result
        .sanitized_input
        .unwrap_or_else(|| result.original_input.clone()))
}

pub async fn secure_optional_request_field(
    security: &dyn InputSecurityPort,
    _field: &str,
    value: Option<&str>,
) -> std::result::Result<Option<String>, SecureInputResult> {
    match value {
        Some(value) if !value.trim().is_empty() => {
            let result = security
                .process_input(value)
                .await
                .unwrap_or_else(|_| SecureInputResult {
                    allowed: false,
                    sanitized_input: None,
                    original_input: value.to_string(),
                    detection_confidence: 1.0,
                    is_injection: true,
                    attack_type: "unknown".to_string(),
                });
            if result.allowed {
                Ok(Some(
                    result
                        .sanitized_input
                        .unwrap_or_else(|| result.original_input.clone()),
                ))
            } else {
                Err(result)
            }
        }
        _ => Ok(None),
    }
}

pub fn secure_cli_input(label: &str, input: &str, max_chars: usize) -> Result<String> {
    let char_count = input.chars().count();
    if char_count > max_chars {
        return Err(anyhow!(
            "{} exceeds maximum length of {} characters",
            label,
            max_chars
        ));
    }

    let security = SecurityService::new();
    let result = security.process_input(input);
    if !result.allowed {
        return Err(anyhow!(
            "{} blocked by security policy: attack_type={}, confidence={:.2}",
            label,
            result.detection.attack_type.as_str(),
            result.detection.confidence
        ));
    }

    if result.sanitized_input.is_some() {
        println!("{} sanitized by security policy before submission.", label);
    }

    Ok(result.effective_input().to_string())
}
