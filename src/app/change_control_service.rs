use crate::domain::change_control::{ChangePolicy, ValidationStatus};

pub struct ChangeControlService {
    policy: ChangePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimResult {
    Approved,
    ApprovalRequired(String),
    Rejected(String),
}

impl ChangeControlService {
    pub fn new(policy: ChangePolicy) -> Self {
        Self { policy }
    }

    pub fn load_default() -> Result<Self, crate::domain::change_control::ChangeControlError> {
        let policy = ChangePolicy::load(".gitcore/change-control.yaml")?;
        Ok(Self::new(policy))
    }

    pub fn validate_claim(&self, _agent_id: &str, resource_path: &str) -> ClaimResult {
        if self.policy.is_critical(resource_path) {
            return ClaimResult::ApprovalRequired(format!(
                "Resource '{}' is critical and requires approval",
                resource_path
            ));
        }

        if let Some((name, rule)) = self.policy.get_layer_for_path(resource_path) {
            if rule.risk == crate::domain::change_control::RiskLevel::High {
                return ClaimResult::ApprovalRequired(format!(
                    "Resource '{}' belongs to high-risk layer '{}' and requires approval",
                    resource_path, name
                ));
            }
        }

        ClaimResult::Approved
    }

    pub fn validate_import(&self, from_path: &str, to_path: &str) -> ValidationStatus {
        self.policy.validate_import(from_path, to_path)
    }

    pub fn policy(&self) -> &ChangePolicy {
        &self.policy
    }
}
