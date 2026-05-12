use anyhow::{anyhow, Result};


use xavier::security::{ProcessResult, SecurityService};

pub fn blocked_external_input_response(label: &str, result: &ProcessResult) -> serde_json::Value {
    serde_json::json!({
        "status": "blocked",
        "blocked": true,
        "reason": "security_policy_violation",
        "message": format!("{label} blocked by security policy"),
        "detection": {
            "is_injection": result.detection.is_injection,
            "confidence": result.detection.confidence,
            "attack_type": result.detection.attack_type.as_str(),
            "message": result.detection.message,
        }
    })
}

pub fn secure_external_input(
    security: &SecurityService,
    label: &str,
    input: &str,
) -> std::result::Result<String, serde_json::Value> {
    let result = security.process_input(input);
    if !result.allowed {
        return Err(blocked_external_input_response(label, &result));
    }

    Ok(result.effective_input().to_string())
}

pub fn secure_optional_request_field(
    security: &SecurityService,
    _field: &str,
    value: Option<&str>,
) -> std::result::Result<Option<String>, ProcessResult> {
    match value {
        Some(value) if !value.trim().is_empty() => {
            let result = security.process_input(value);
            if result.allowed {
                Ok(Some(result.effective_input().to_string()))
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
