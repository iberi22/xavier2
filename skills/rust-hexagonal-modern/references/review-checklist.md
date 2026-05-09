# Review Checklist

Use this checklist when reviewing or tightening Rust code with this skill.

- Does the module belong in `domain`, `application`, `ports`, or `adapters`, and is that placement explicit?
- Do domain types avoid HTTP, SQL, filesystem, and framework-specific dependencies?
- Are handlers, CLI commands, or UI callbacks thin translators instead of business-logic containers?
- Are outbound integrations hidden behind ports or another clearly owned boundary?
- Is async work owned and cancellable, or is the code relying on detached tasks without lifecycle control?
- Are `reqwest::Client`, DB handles, and shared state reused instead of rebuilt per call?
- Are errors modeled with enough structure to preserve meaning without leaking transport details upward?
- Are logs structured and placed at the right boundaries?
- Do tests cover the cheapest layer that can prove the behavior?
- Did the change introduce a new crate for convenience when an existing crate or pattern already fits?
