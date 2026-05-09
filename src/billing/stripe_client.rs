//! Stripe API client for billing operations.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use super::plans::{Plan, PlanLimits, SubscriptionStatus};

/// Stripe API client
#[derive(Clone)]
pub struct StripeClient {
    http_client: Client,
    secret_key: String,
    webhook_secret: String,
}

impl StripeClient {
    /// Create a new Stripe client from environment variables
    pub fn from_env() -> Result<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| anyhow!("STRIPE_SECRET_KEY not configured"))?;
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| anyhow!("STRIPE_WEBHOOK_SECRET not configured"))?;

        Ok(Self {
            http_client: Client::new(),
            secret_key,
            webhook_secret,
        })
    }

    /// Check if Stripe billing is configured
    pub fn is_configured() -> bool {
        std::env::var("STRIPE_SECRET_KEY").is_ok()
    }

    /// Get the base URL for Stripe API
    fn base_url(&self) -> &str {
        "https://api.stripe.com/v1"
    }

    /// Create a new Stripe customer
    pub async fn create_customer(&self, email: &str, name: &str) -> Result<Customer> {
        let url = format!("{}/customers", self.base_url());

        let params = [
            ("email", email),
            ("name", name),
        ];

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.secret_key, Some(""))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Stripe create_customer failed: {} - {}", status, body);
            return Err(anyhow!("Stripe API error: {} - {}", status, body));
        }

        let customer: Customer = response.json().await?;
        info!("Created Stripe customer: {}", customer.id);
        Ok(customer)
    }

    /// Create a checkout session for subscription
    pub async fn create_checkout_session(
        &self,
        customer_id: &str,
        price_id: &str,
        success_url: &str,
        cancel_url: &str,
        workspace_id: &str,
    ) -> Result<CheckoutSession> {
        let url = format!("{}/checkout/sessions", self.base_url());

        let params = vec![
            ("customer", customer_id),
            ("mode", "subscription"),
            ("payment_method_types[]", "card"),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("metadata[workspace_id]", workspace_id),
        ];

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.secret_key, Some(""))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Stripe create_checkout_session failed: {} - {}", status, body);
            return Err(anyhow!("Stripe API error: {} - {}", status, body));
        }

        let session: CheckoutSession = response.json().await?;
        info!("Created checkout session: {}", session.id);
        Ok(session)
    }

    /// Create a customer portal session
    pub async fn create_portal_session(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> Result<PortalSession> {
        let url = format!("{}/billing_portal/sessions", self.base_url());

        let params = [
            ("customer", customer_id),
            ("return_url", return_url),
        ];

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.secret_key, Some(""))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Stripe create_portal_session failed: {} - {}", status, body);
            return Err(anyhow!("Stripe API error: {} - {}", status, body));
        }

        let session: PortalSession = response.json().await?;
        info!("Created portal session for customer: {}", customer_id);
        Ok(session)
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(&self, subscription_id: &str) -> Result<Subscription> {
        let url = format!(
            "{}/subscriptions/{}",
            self.base_url(),
            urlencoding::encode(subscription_id)
        );

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.secret_key, Some(""))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Stripe cancel_subscription failed: {} - {}", status, body);
            return Err(anyhow!("Stripe API error: {} - {}", status, body));
        }

        let subscription: Subscription = response.json().await?;
        info!("Cancelled subscription: {}", subscription_id);
        Ok(subscription)
    }

    /// Get subscription details
    pub async fn get_subscription(&self, subscription_id: &str) -> Result<Subscription> {
        let url = format!(
            "{}/subscriptions/{}",
            self.base_url(),
            urlencoding::encode(subscription_id)
        );

        let response = self
            .http_client
            .get(&url)
            .basic_auth(&self.secret_key, Some(""))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Stripe get_subscription failed: {} - {}", status, body);
            return Err(anyhow!("Stripe API error: {} - {}", status, body));
        }

        let subscription: Subscription = response.json().await?;
        Ok(subscription)
    }

    /// Verify webhook signature
    pub fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent> {
        // Parse the signature header
        let parts: Vec<&str> = signature.split(',').collect();
        let mut timestamp: Option<&str> = None;
        let mut sig: Option<&str> = None;

        for part in parts {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "t" => timestamp = Some(kv[1]),
                    "v1" => sig = Some(kv[1]),
                    _ => {}
                }
            }
        }

        let timestamp = timestamp.ok_or_else(|| anyhow!("Missing timestamp in signature"))?;
        let sig = sig.ok_or_else(|| anyhow!("Missing v1 signature"))?;

        // Compute expected signature
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        // Use HMAC SHA256
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .map_err(|_| anyhow!("HMAC error"))?;
        mac.update(signed_payload.as_bytes());

        let expected = hex::encode(mac.finalize().into_bytes());

        if sig != expected {
            return Err(anyhow!("Webhook signature verification failed"));
        }

        // Parse the event
        let event: WebhookEvent = serde_json::from_slice(payload)?;
        Ok(event)
    }
}

// ============================================================================
// Stripe API response types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: Option<String>,
    pub customer: String,
    pub mode: String,
    pub status: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalSession {
    pub id: String,
    pub url: String,
    pub customer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub current_period_start: i64,
    pub current_period_end: i64,
    #[serde(default)]
    pub items: SubscriptionItems,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscriptionItems {
    #[serde(default)]
    pub data: Vec<SubscriptionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionItem {
    pub id: String,
    pub price: Price,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub data: WebhookEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookEventData {
    #[serde(default)]
    pub object: serde_json::Value,
}

// ============================================================================
// Billing service for workspace integration
// ============================================================================

/// Billing service that manages workspace subscriptions
#[derive(Clone)]
pub struct BillingService {
    stripe: StripeClient,
}

impl BillingService {
    /// Create a new billing service
    pub fn new() -> Result<Self> {
        Ok(Self {
            stripe: StripeClient::from_env()?,
        })
    }

    /// Check if billing is available
    pub fn is_available() -> bool {
        StripeClient::is_configured()
    }

    /// Get subscription status for a workspace
    pub async fn get_status(&self, workspace_id: &str) -> Result<SubscriptionStatus> {
        // Load workspace billing metadata from store
        let metadata = self.get_workspace_billing_metadata(workspace_id).await?;

        if let Some(stripe_customer_id) = &metadata.stripe_customer_id {
            if let Some(subscription_id) = &metadata.stripe_subscription_id {
                match self.stripe.get_subscription(subscription_id).await {
                    Ok(sub) => {
                        let plan = Plan::from_price_id(
                            sub.items.data.first()
                                .map(|item| item.price.id.as_str())
                                .unwrap_or("")
                        ).unwrap_or(Plan::Free);

                        let subscription_status = sub.status.clone();
                        let is_active = subscription_status == "active" || subscription_status == "trialing";

                        return Ok(SubscriptionStatus {
                            plan,
                            stripe_customer_id: Some(stripe_customer_id.clone()),
                            stripe_subscription_id: Some(subscription_id.clone()),
                            subscription_status,
                            current_period_end: Some(sub.current_period_end),
                            is_active,
                            limits: PlanLimits::for_plan(plan),
                        });
                    }
                    Err(e) => {
                        info!("Could not fetch subscription {}: {}", subscription_id, e);
                    }
                }
            }
        }

        Ok(SubscriptionStatus::default())
    }

    /// Create a new customer for a workspace
    pub async fn create_customer(&self, workspace_id: &str, email: &str, name: &str) -> Result<String> {
        let customer = self.stripe.create_customer(email, name).await?;

        // Store customer ID in workspace metadata
        self.save_workspace_billing_metadata(workspace_id, &WorkspaceBillingMetadata {
            stripe_customer_id: Some(customer.id.clone()),
            stripe_subscription_id: None,
            stripe_price_id: None,
        }).await?;

        Ok(customer.id)
    }

    /// Create checkout session for upgrading
    pub async fn create_checkout(
        &self,
        workspace_id: &str,
        plan: Plan,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String> {
        let metadata = self.get_workspace_billing_metadata(workspace_id).await?;

        let customer_id = metadata.stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer found. Call create-customer first."))?;

        let price_id = plan.price_id()
            .ok_or_else(|| anyhow!("Plan {} does not have a price configured", plan))?;

        let session = self.stripe
            .create_checkout_session(&customer_id, &price_id, success_url, cancel_url, workspace_id)
            .await?;

        // Store price ID for later reference
        self.save_workspace_billing_metadata(workspace_id, &WorkspaceBillingMetadata {
            stripe_customer_id: Some(customer_id),
            stripe_subscription_id: None,
            stripe_price_id: Some(price_id),
        }).await?;

        Ok(session.url.unwrap_or_default())
    }

    /// Create customer portal session
    pub async fn create_portal(&self, workspace_id: &str, return_url: &str) -> Result<String> {
        let metadata = self.get_workspace_billing_metadata(workspace_id).await?;

        let customer_id = metadata.stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer found"))?;

        let session = self.stripe.create_portal_session(&customer_id, return_url).await?;
        Ok(session.url)
    }

    /// Cancel subscription
    pub async fn cancel_subscription(&self, workspace_id: &str) -> Result<()> {
        let metadata = self.get_workspace_billing_metadata(workspace_id).await?;

        let subscription_id = metadata.stripe_subscription_id
            .ok_or_else(|| anyhow!("No active subscription found"))?;

        self.stripe.cancel_subscription(&subscription_id).await?;

        // Clear subscription metadata
        self.save_workspace_billing_metadata(workspace_id, &WorkspaceBillingMetadata {
            stripe_customer_id: metadata.stripe_customer_id,
            stripe_subscription_id: None,
            stripe_price_id: None,
        }).await?;

        Ok(())
    }

    async fn get_workspace_billing_metadata(&self, _workspace_id: &str) -> Result<WorkspaceBillingMetadata> {
        // This would load from workspace metadata store
        // For now, return empty metadata
        Ok(WorkspaceBillingMetadata::default())
    }

    async fn save_workspace_billing_metadata(&self, workspace_id: &str, metadata: &WorkspaceBillingMetadata) -> Result<()> {
        // This would save to workspace metadata store
        // For now, just log
        info!("Would save billing metadata for workspace {}: {:?}", workspace_id, metadata);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceBillingMetadata {
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub stripe_price_id: Option<String>,
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for c in s.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    encoded.push(c);
                }
                _ => {
                    for byte in c.to_string().as_bytes() {
                        encoded.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding::encode("sub_123"), "sub_123");
        assert_eq!(urlencoding::encode("cus_abc/def"), "cus_abc%2Fdef");
    }

    #[test]
    fn test_workspace_billing_metadata_default() {
        let meta = WorkspaceBillingMetadata::default();
        assert!(meta.stripe_customer_id.is_none());
        assert!(meta.stripe_subscription_id.is_none());
    }
}
