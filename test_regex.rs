use regex::Regex;

fn main() {
    let slack_pattern = Regex::new(r"xox[baprs]-[a-zA-Z0-9]{20,}").unwrap();

    // Should NOT match (length 10)
    let too_short = "xoxb-1234567890";
    assert!(!slack_pattern.is_match(too_short), "Should not match 10 chars");

    // Should match (length 20)
    let just_right = "xoxb-12345678901234567890";
    assert!(slack_pattern.is_match(just_right), "Should match 20 chars");

    // Should match (length > 20)
    let long_enough = "xoxb-1234567890123456789012345";
    assert!(slack_pattern.is_match(long_enough), "Should match > 20 chars");

    println!("Regex verification successful!");
}
