//! Billing module for Stripe integration.
//!
//! Provides cloud tier billing for Xavier2 with the following pricing:
//! - Free: $0/mo - Local only
//! - Cloud: $8/mo - 1GB storage, 3 nodes
//! - Pro: $19/mo - 10GB storage, 10 nodes
//! - Enterprise: $49/mo+ - Unlimited

pub mod plans;
pub mod stripe_client;
pub mod webhook;

use axum::{
    extract::Extension,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::workspace::WorkspaceContext;
use plans::{Plan, PlanLimits, SubscriptionStatus};
use stripe_client::BillingService;

/// Billing configuration from environment
#[derive(Debug, Clone)]
pub struct BillingConfig {
    pub enabled: bool,
    pub success_url: String,
    pub cancel_url: String,
}

impl BillingConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: stripe_client::StripeClient::is_configured(),
            success_url: std::env::var("STRIPE_SUCCESS_URL")
                .unwrap_or_else(|_| "https://xavier2.example.com/billing/success".to_string()),
            cancel_url: std::env::var("STRIPE_CANCEL_URL")
                .unwrap_or_else(|_| "https://xavier2.example.com/billing/cancel".to_string()),
        }
    }
}

// ============================================================================
// HTTP Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCustomerResponse {
    pub status: String,
    pub customer_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    pub status: String,
    pub checkout_url: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePortalRequest {
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatePortalResponse {
    pub status: String,
    pub portal_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CancelSubscriptionResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BillingStatusResponse {
    pub status: String,
    #[serde(flatten)]
    pub subscription: SubscriptionStatus,
}

#[derive(Debug, Serialize)]
pub struct BillingPlansResponse {
    pub status: String,
    pub plans: Vec<PlanInfo>,
}

#[derive(Debug, Serialize)]
pub struct PlanInfo {
    pub name: String,
    pub display_name: String,
    pub monthly_price_cents: u32,
    pub limits: PlanLimits,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Create a new Stripe customer for the workspace
pub async fn create_customer(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<CreateCustomerRequest>,
) -> impl IntoResponse {
    if !BillingService::is_available() {
        return Json(CreateCustomerResponse {
            status: "error".to_string(),
            customer_id: String::new(),
        });
    }

    match BillingService::new() {
        Ok(billing) => match billing.create_customer(&workspace.workspace_id, &payload.email, &payload.name).await {
            Ok(customer_id) => Json(CreateCustomerResponse {
                status: "ok".to_string(),
                customer_id,
            }),
            Err(e) => Json(CreateCustomerResponse {
                status: format!("error: {}", e),
                customer_id: String::new(),
            }),
        },
        Err(e) => Json(CreateCustomerResponse {
            status: format!("error: {}", e),
            customer_id: String::new(),
        }),
    }
}

/// Create a checkout session for subscription upgrade
pub async fn create_checkout(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<CreateCheckoutRequest>,
) -> impl IntoResponse {
    if !BillingService::is_available() {
        return Json(CreateCheckoutResponse {
            status: "error".to_string(),
            checkout_url: None,
            message: Some("Billing not configured".to_string()),
        });
    }

    let plan = match payload.plan.to_lowercase().as_str() {
        "cloud" => Plan::Cloud,
        "pro" => Plan::Pro,
        "enterprise" => Plan::Enterprise,
        _ => {
            return Json(CreateCheckoutResponse {
                status: "error".to_string(),
                checkout_url: None,
                message: Some("Invalid plan. Use: cloud, pro, or enterprise".to_string()),
            });
        }
    };

    let config = BillingConfig::from_env();

    match BillingService::new() {
        Ok(billing) => {
            match billing.create_checkout(
                &workspace.workspace_id,
                plan,
                &config.success_url,
                &config.cancel_url,
            ).await {
                Ok(url) => Json(CreateCheckoutResponse {
                    status: "ok".to_string(),
                    checkout_url: Some(url),
                    message: None,
                }),
                Err(e) => Json(CreateCheckoutResponse {
                    status: format!("error: {}", e),
                    checkout_url: None,
                    message: None,
                }),
            }
        }
        Err(e) => Json(CreateCheckoutResponse {
            status: format!("error: {}", e),
            checkout_url: None,
            message: None,
        }),
    }
}

/// Create a customer portal session
pub async fn create_portal(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<CreatePortalRequest>,
) -> impl IntoResponse {
    if !BillingService::is_available() {
        return Json(CreatePortalResponse {
            status: "error".to_string(),
            portal_url: None,
        });
    }

    let config = BillingConfig::from_env();
    let return_url = payload.return_url.unwrap_or(config.cancel_url);

    match BillingService::new() {
        Ok(billing) => {
            match billing.create_portal(&workspace.workspace_id, &return_url).await {
                Ok(url) => Json(CreatePortalResponse {
                    status: "ok".to_string(),
                    portal_url: Some(url),
                }),
                Err(e) => Json(CreatePortalResponse {
                    status: format!("error: {}", e),
                    portal_url: None,
                }),
            }
        }
        Err(e) => Json(CreatePortalResponse {
            status: format!("error: {}", e),
            portal_url: None,
        }),
    }
}

/// Cancel subscription
pub async fn cancel_subscription(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    if !BillingService::is_available() {
        return Json(CancelSubscriptionResponse {
            status: "error".to_string(),
            message: "Billing not configured".to_string(),
        });
    }

    match BillingService::new() {
        Ok(billing) => {
            match billing.cancel_subscription(&workspace.workspace_id).await {
                Ok(()) => Json(CancelSubscriptionResponse {
                    status: "ok".to_string(),
                    message: "Subscription cancelled".to_string(),
                }),
                Err(e) => Json(CancelSubscriptionResponse {
                    status: format!("error: {}", e),
                    message: e.to_string(),
                }),
            }
        }
        Err(e) => Json(CancelSubscriptionResponse {
            status: format!("error: {}", e),
            message: e.to_string(),
        }),
    }
}

/// Get current billing status
pub async fn billing_status(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    // If billing is not configured, return free tier status
    if !BillingService::is_available() {
        let status = SubscriptionStatus {
            plan: Plan::Free,
            limits: PlanLimits::for_plan(Plan::Free),
            ..Default::default()
        };
        return Json(BillingStatusResponse {
            status: "ok".to_string(),
            subscription: status,
        });
    }

    match BillingService::new() {
        Ok(billing) => {
            match billing.get_status(&workspace.workspace_id).await {
                Ok(subscription) => Json(BillingStatusResponse {
                    status: "ok".to_string(),
                    subscription,
                }),
                Err(e) => {
                    // Return free tier on error
                    let status = SubscriptionStatus {
                        plan: Plan::Free,
                        limits: PlanLimits::for_plan(Plan::Free),
                        ..Default::default()
                    };
                    Json(BillingStatusResponse {
                        status: format!("error: {}", e),
                        subscription: status,
                    })
                }
            }
        }
        Err(_) => {
            let status = SubscriptionStatus {
                plan: Plan::Free,
                limits: PlanLimits::for_plan(Plan::Free),
                ..Default::default()
            };
            Json(BillingStatusResponse {
                status: "error".to_string(),
                subscription: status,
            })
        }
    }
}

/// Get available billing plans
pub async fn billing_plans() -> impl IntoResponse {
    let plans = vec![
        PlanInfo {
            name: "free".to_string(),
            display_name: "Free".to_string(),
            monthly_price_cents: 0,
            limits: PlanLimits::for_plan(Plan::Free),
        },
        PlanInfo {
            name: "cloud".to_string(),
            display_name: "Cloud".to_string(),
            monthly_price_cents: Plan::Cloud.monthly_price_cents(),
            limits: PlanLimits::for_plan(Plan::Cloud),
        },
        PlanInfo {
            name: "pro".to_string(),
            display_name: "Pro".to_string(),
            monthly_price_cents: Plan::Pro.monthly_price_cents(),
            limits: PlanLimits::for_plan(Plan::Pro),
        },
        PlanInfo {
            name: "enterprise".to_string(),
            display_name: "Enterprise".to_string(),
            monthly_price_cents: Plan::Enterprise.monthly_price_cents(),
            limits: PlanLimits::for_plan(Plan::Enterprise),
        },
    ];

    Json(BillingPlansResponse {
        status: "ok".to_string(),
        plans,
    })
}
