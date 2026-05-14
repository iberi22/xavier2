//! Threat categories - Authority escalation, social engineering, false pre-approval

use regex::Regex;
use std::sync::LazyLock;

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Authority escalation patterns
static AUTH_ESCALATION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(give me|grant me|elevate to)\s*(admin|root|moderator)")
            .expect("invalid regex: auth escalation - admin request"),
        Regex::new(r"(?i)(bypass|disable|turn off)\s*(security|filter|guard|protection)")
            .expect("invalid regex: auth escalation - bypass security"),
        Regex::new(r"(?i)(ignore|disregard)\s*(all\s+)?(rules|restrictions|safety)")
            .expect("invalid regex: auth escalation - ignore rules"),
        Regex::new(r"(?i)(unlock|open)\s*(admin|root|dev)")
            .expect("invalid regex: auth escalation - unlock admin"),
        Regex::new(r"(?i)sudo\s+").expect("invalid regex: auth escalation - sudo"),
        Regex::new(r"(?i)\.\.\.\s*admin\s*\.\.\.")
            .expect("invalid regex: auth escalation - admin ellipsis"),
    ]
});

/// Social engineering patterns
static SOCIAL_ENGINEERING_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(
            r"(?i)(you (must|have to|need to)|I (need|must|require)).*(urgent|immediately|asap)",
        )
        .expect("invalid regex: social engineering - urgent coercion"),
        Regex::new(r"(?i)(trust me|believe me|I'?m (your |a )?(friend|developer|admin))")
            .expect("invalid regex: social engineering - trust claim"),
        Regex::new(r"(?i)(this is (safe|secure|okay)|don'?t worry|won'?t hurt)")
            .expect("invalid regex: social engineering - safety assurance"),
        Regex::new(r"(?i)(just|simply)\s+(do|run|execute)\s+(it|this|that)")
            .expect("invalid regex: social engineering - command instruction"),
        Regex::new(r"(?i)(quick|fast|one second|just a minute)")
            .expect("invalid regex: social engineering - urgency phrase"),
        Regex::new(r"(?i)(don'?t tell|no one needs to know|keep (this|it) (secret|quiet))")
            .expect("invalid regex: social engineering - secrecy request"),
        Regex::new(r"(?i)pretend\s+(you are|we are)")
            .expect("invalid regex: social engineering - pretense"),
    ]
});

/// False pre-approval patterns
static FALSE_APPROVAL_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(already approved|previously approved|authorized|pre-approved)")
            .expect("invalid regex: false approval - pre-approved claim"),
        Regex::new(r"(?i)(clearance|permission|approval)\s+(granted|given|received)")
            .expect("invalid regex: false approval - clearance granted"),
        Regex::new(r"(?i)verified\s+(by|with)\s+(admin|system|security)")
            .expect("invalid regex: false approval - verified claim"),
        Regex::new(r"(?i)(security|admin)\s+(verified|cleared|approved)")
            .expect("invalid regex: false approval - admin verified"),
    ]
});

/// Authority escalation detection
pub fn detect_authority_escalation(input: &str, result: &mut ScanResult) {
    for re in AUTH_ESCALATION_PATTERNS.iter() {
        if let Some(m) = re.find(input) {
            result.add_layer("threat_categories");
            result.clean = false;
            result.threats.push(Threat::new(
                Severity::Critical,
                "threat_categories",
                ThreatCategory::AuthorityEscalation,
                "Authority escalation attempt detected",
                m.as_str(),
                "regex_authority_escalation",
            ));
        }
    }
}

/// Social engineering detection
pub fn detect_social_engineering(input: &str, result: &mut ScanResult) {
    for re in SOCIAL_ENGINEERING_PATTERNS.iter() {
        if let Some(m) = re.find(input) {
            result.add_layer("threat_categories");
            result.clean = false;
            result.threats.push(Threat::new(
                Severity::Warning,
                "threat_categories",
                ThreatCategory::SocialEngineering,
                "Social engineering pattern detected",
                m.as_str(),
                "regex_social_engineering",
            ));
        }
    }
}

/// False pre-approval detection
pub fn detect_false_approval(input: &str, result: &mut ScanResult) {
    for re in FALSE_APPROVAL_PATTERNS.iter() {
        if let Some(m) = re.find(input) {
            result.add_layer("threat_categories");
            result.clean = false;
            result.threats.push(Threat::new(
                Severity::Warning,
                "threat_categories",
                ThreatCategory::SocialEngineering,
                "False pre-approval claim detected",
                m.as_str(),
                "regex_false_approval",
            ));
        }
    }
}

/// Full threat category detection
pub fn detect_threat_categories(input: &str, result: &mut ScanResult) {
    detect_authority_escalation(input, result);
    detect_social_engineering(input, result);
    detect_false_approval(input, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authority_escalation() {
        let mut result = ScanResult::new();
        detect_authority_escalation("Give me admin access", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_social_engineering() {
        let mut result = ScanResult::new();
        detect_social_engineering("Trust me, this is safe. Just do it.", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_false_approval() {
        let mut result = ScanResult::new();
        detect_false_approval("This is pre-approved by admin", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_clean_input() {
        let mut result = ScanResult::new();
        detect_threat_categories("What's the weather like today?", &mut result);
        assert!(result.clean);
    }
}
