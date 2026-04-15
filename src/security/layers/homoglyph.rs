//! Homoglyph detection - Unicode normalization and lookalike detection

use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Latin/Cyrillic lookalike pairs
const LOOKALIKE_PAIRS: &[(char, char)] = &[
    ('a', 'а'), // Latin a vs Cyrillic а
    ('e', 'е'), // Latin e vs Cyrillic е
    ('o', 'о'), // Latin o vs Cyrillic о
    ('p', 'р'), // Latin p vs Cyrillic р
    ('c', 'с'), // Latin c vs Cyrillic с
    ('y', 'у'), // Latin y vs Cyrillic у
    ('x', 'х'), // Latin x vs Cyrillic х
    ('k', 'к'), // Latin k vs Cyrillic к
    ('m', 'м'), // Latin m vs Cyrillic м
    ('t', 'т'), // Latin t vs Cyrillic т
    ('b', 'в'), // Latin b vs Cyrillic в
];

/// Detect Unicode normalization issues (potential homoglyph attack)
pub fn detect_unicode_normalization(input: &str, result: &mut ScanResult) {
    let normalized = input.nfc().collect::<String>();

    if normalized != input {
        result.add_layer("homoglyph");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "homoglyph",
            ThreatCategory::HomoglyphSpoofing,
            "Unicode normalization difference detected",
            "non_normalized_unicode",
            "unicode_nfc_normalization",
        ));
    }
}

/// Detect mixed Latin/Cyrillic characters
pub fn detect_mixed_scripts(input: &str, result: &mut ScanResult) {
    let mut latin_count = 0;
    let mut cyrillic_count = 0;
    let mut has_mixed = false;

    for c in input.chars() {
        if c.is_ascii_alphabetic() {
            latin_count += 1;
        } else if matches!(c as u32, 0x0410..=0x044F) {
            // Cyrillic А-Я (uppercase and lowercase)
            cyrillic_count += 1;
            has_mixed = true;
        }
    }

    if has_mixed && latin_count > 0 && cyrillic_count > 0 {
        result.add_layer("homoglyph");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "homoglyph",
            ThreatCategory::HomoglyphSpoofing,
            "Mixed Latin/Cyrillic scripts detected",
            &format!("Latin: {}, Cyrillic: {}", latin_count, cyrillic_count),
            "script_mixing_detection",
        ));
    }
}

/// Detect Latin/Cyrillic lookalikes
pub fn detect_lookalikes(input: &str, result: &mut ScanResult) {
    let input_lower = input.to_lowercase();
    let mut found_pairs: HashSet<String> = HashSet::new();

    for (latin, cyrillic) in LOOKALIKE_PAIRS {
        let latin_count = input_lower.chars().filter(|c| *c == *latin).count();
        let cyrillic_count = input_lower.chars().filter(|c| *c == *cyrillic).count();

        // If both lookalikes present, it's suspicious
        if latin_count > 0 && cyrillic_count > 0 && latin_count + cyrillic_count >= 4 {
            let pair_str = format!("{}/{}", latin, cyrillic);
            if found_pairs.insert(pair_str) {
                result.add_layer("homoglyph");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Warning,
                    "homoglyph",
                    ThreatCategory::HomoglyphSpoofing,
                    &format!("Latin '{}' and Cyrillic '{}' both present", latin, cyrillic),
                    &format!("latin:{}, cyrillic:{}", latin_count, cyrillic_count),
                    "lookalike_detection",
                ));
            }
        }
    }
}

/// Detect RTL override characters
pub fn detect_rtl_override(input: &str, result: &mut ScanResult) {
    if input.contains('\u{202E}') || input.contains('\u{202B}') {
        result.add_layer("homoglyph");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Critical,
            "homoglyph",
            ThreatCategory::HomoglyphSpoofing,
            "RTL override character detected",
            "U+202E or U+202B",
            "rtl_override_detection",
        ));
    }
}

/// Detect zero-width characters (beyond those in heuristic)
pub fn detect_zero_width(input: &str, result: &mut ScanResult) {
    let zero_width_chars = [
        '\u{200B}', // Zero Width Space
        '\u{200C}', // Zero Width Non-Joiner
        '\u{200D}', // Zero Width Joiner
        '\u{FEFF}', // Byte Order Mark
        '\u{180E}', // Mongolian Vowel Separator (deprecated but still used)
        '\u{2060}', // Word Joiner
    ];

    for c in zero_width_chars {
        if input.contains(c) {
            result.add_layer("homoglyph");
            result.clean = false;
            result.threats.push(Threat::new(
                Severity::Warning,
                "homoglyph",
                ThreatCategory::EncodingAttack,
                &format!("Zero-width character U+{:04X} detected", c as u32),
                &format!("\\u{:04X}", c as u32),
                "zero_width_unicode",
            ));
        }
    }
}

/// Run all homoglyph detections
pub fn detect_homoglyph(input: &str, result: &mut ScanResult) {
    detect_unicode_normalization(input, result);
    detect_mixed_scripts(input, result);
    detect_lookalikes(input, result);
    detect_rtl_override(input, result);
    detect_zero_width(input, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixed_scripts() {
        let mut result = ScanResult::new();
        detect_mixed_scripts("Hello wоrld", &mut result); // о is Cyrillic
        assert!(!result.clean);
    }

    #[test]
    fn test_lookalikes() {
        let mut result = ScanResult::new();
        // Mix of Latin 'a' and Cyrillic 'а'
        detect_lookalikes("pаypаl", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_rtl_override() {
        let mut result = ScanResult::new();
        detect_rtl_override("Hello\u{202E}World", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_clean_text() {
        let mut result = ScanResult::new();
        detect_homoglyph("Hello world, how are you?", &mut result);
        assert!(result.clean);
    }

    #[test]
    fn test_unicode_normalization() {
        let mut result = ScanResult::new();
        // Composed vs decomposed forms
        detect_unicode_normalization("café", &mut result);
        // May or may not trigger depending on normalization form
    }
}
