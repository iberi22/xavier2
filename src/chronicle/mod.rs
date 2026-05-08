//! Chronicle Module
//!
//! Chronicle provides security and data integrity layers for Xavier2.
//! It includes automated redaction of sensitive information.

pub mod patterns;
pub mod redact;

pub use redact::{process_output, redact, verify};
