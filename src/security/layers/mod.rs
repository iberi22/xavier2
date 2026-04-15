//! Security layers module

pub mod canary;
pub mod config_drift;
pub mod encoding;
pub mod entropy;
pub mod heuristic;
pub mod homoglyph;
pub mod path_traversal;
pub mod phrase;
pub mod threat_categories;
pub mod tool_alias;

pub use canary::detect_canary;
pub use config_drift::detect_config_drift_full;
pub use encoding::detect_encoding_attacks;
pub use entropy::{detect_high_entropy, detect_secrets, shannon_entropy};
pub use heuristic::detect_heuristic;
pub use homoglyph::detect_homoglyph;
pub use path_traversal::detect_path_traversal;
pub use phrase::{contains_injection, find_matches, get_match_positions};
pub use threat_categories::detect_threat_categories;
pub use tool_alias::detect_tool_alias_full;
