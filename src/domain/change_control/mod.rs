pub mod conflict;
pub mod lease;
pub mod scope;
pub mod task;

pub use conflict::{ConflictReport, ConflictType};
pub use lease::{FileLease, LeaseMode, LeaseStatus};
pub use scope::ChangeScope;
pub use task::{AgentTask, AgentTaskStatus};

/// List of high-risk paths that require strict change control.
pub const CRITICAL_FILES: &[&str] = &[
    "Cargo.toml",
    "src/lib.rs",
    "src/main.rs",
    "src/server/mod.rs",
    "src/domain/mod.rs",
];
