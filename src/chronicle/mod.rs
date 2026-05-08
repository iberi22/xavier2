//! Chronicle module.
//!
//! Chronicle provides harvesting, redaction, generation, and publishing
//! primitives for Xavier's daily technical log workflow.

pub mod generate;
pub mod harvest;
pub mod patterns;
pub mod prompts;
pub mod redact;

pub use redact::{process_output, redact, verify};
