//! Plan definitions and limits for Xavier2 billing tiers.

use serde::{Deserialize, Serialize};

/// Billing plan tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// Free tier - local only, no cloud features
    Free,
    /// Cloud tier - 1GB storage, 3 nodes
    Cloud,
    /// Pro tier - 10GB storage, 10 nodes
    Pro,
    /// Enterprise tier - unlimited, custom pricing
    Enterprise,
}

impl Plan {
    /// Get plan from Stripe price ID
    pub fn from_price_id(price_id: &str) -> Option<Self> {
        let cloud_price = std::env::var("STRIPE_PRICE_CLOUD").ok()?;
        let pro_price = std::env::var("STRIPE_PRICE_PRO").ok()?;
        let enterprise_price = std::env::var("STRIPE_PRICE_ENTERPRISE").ok()?;

        if price_id == cloud_price {
            Some(Self::Cloud)
        } else if price_id == pro_price {
            Some(Self::Pro)
        } else if price_id == enterprise_price {
            Some(Self::Enterprise)
        } else {
            None
        }
    }

    /// Convert plan to Stripe price ID
    pub fn price_id(&self) -> Option<String> {
        match self {
            Self::Free => None,
            Self::Cloud => std::env::var("STRIPE_PRICE_CLOUD").ok(),
            Self::Pro => std::env::var("STRIPE_PRICE_PRO").ok(),
            Self::Enterprise => std::env::var("STRIPE_PRICE_ENTERPRISE").ok(),
        }
    }

    /// Monthly price in cents
    pub fn monthly_price_cents(&self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Cloud => 800,  // $8.00
            Self::Pro => 1900,   // $19.00
            Self::Enterprise => 4900, // $49.00
        }
    }
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Free => write!(f, "free"),
            Self::Cloud => write!(f, "cloud"),
            Self::Pro => write!(f, "pro"),
            Self::Enterprise => write!(f, "enterprise"),
        }
    }
}

/// Plan limits and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    /// Maximum storage in GB (0 for unlimited)
    pub max_storage_gb: usize,
    /// Maximum number of nodes (0 for unlimited)
    pub max_nodes: usize,
    /// List of feature flags enabled for this plan
    pub features: Vec<String>,
}

impl PlanLimits {
    /// Get limits for a specific plan
    pub fn for_plan(plan: Plan) -> Self {
        match plan {
            Plan::Free => Self {
                max_storage_gb: 0,
                max_nodes: 0,
                features: vec![
                    "local_only".to_string(),
                    "basic_memory".to_string(),
                ],
            },
            Plan::Cloud => Self {
                max_storage_gb: 1,
                max_nodes: 3,
                features: vec![
                    "cloud_sync".to_string(),
                    "basic_memory".to_string(),
                    "api_access".to_string(),
                    "email_support".to_string(),
                ],
            },
            Plan::Pro => Self {
                max_storage_gb: 10,
                max_nodes: 10,
                features: vec![
                    "cloud_sync".to_string(),
                    "advanced_memory".to_string(),
                    "api_access".to_string(),
                    "priority_support".to_string(),
                    "advanced_analytics".to_string(),
                    "team_sharing".to_string(),
                ],
            },
            Plan::Enterprise => Self {
                max_storage_gb: 0, // Unlimited
                max_nodes: 0,      // Unlimited
                features: vec![
                    "cloud_sync".to_string(),
                    "advanced_memory".to_string(),
                    "api_access".to_string(),
                    "dedicated_support".to_string(),
                    "advanced_analytics".to_string(),
                    "team_sharing".to_string(),
                    "sso".to_string(),
                    "custom_integrations".to_string(),
                    "sla".to_string(),
                ],
            },
        }
    }

    /// Check if a feature is enabled for this plan
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

/// Current subscription status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStatus {
    /// Current plan
    pub plan: Plan,
    /// Stripe customer ID
    pub stripe_customer_id: Option<String>,
    /// Stripe subscription ID
    pub stripe_subscription_id: Option<String>,
    /// Subscription status from Stripe
    pub subscription_status: String,
    /// Current period end timestamp
    pub current_period_end: Option<i64>,
    /// Whether the subscription is active
    pub is_active: bool,
    /// Plan limits
    pub limits: PlanLimits,
}

impl Default for SubscriptionStatus {
    fn default() -> Self {
        Self {
            plan: Plan::Free,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            subscription_status: "none".to_string(),
            current_period_end: None,
            is_active: false,
            limits: PlanLimits::for_plan(Plan::Free),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_from_price_id() {
        // This test requires env vars to be set
        let result = Plan::from_price_id("price_nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_plan_limits() {
        let free_limits = PlanLimits::for_plan(Plan::Free);
        assert_eq!(free_limits.max_storage_gb, 0);
        assert!(free_limits.has_feature("local_only"));

        let cloud_limits = PlanLimits::for_plan(Plan::Cloud);
        assert_eq!(cloud_limits.max_storage_gb, 1);
        assert!(cloud_limits.has_feature("cloud_sync"));

        let pro_limits = PlanLimits::for_plan(Plan::Pro);
        assert_eq!(pro_limits.max_storage_gb, 10);
        assert!(pro_limits.has_feature("advanced_analytics"));
    }

    #[test]
    fn test_subscription_status_default() {
        let status = SubscriptionStatus::default();
        assert_eq!(status.plan, Plan::Free);
        assert!(!status.is_active);
    }
}
