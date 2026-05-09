//! Stripe webhook handling.

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use http_body_util::BodyExt;
use tracing::{error, info, warn};

use super::stripe_client::{StripeClient, WebhookEvent};
use crate::AppState;

/// Webhook handler for Stripe events
pub async fn handle_webhook(
    State(state): State<AppState>,
    mut request: Request<Body>,
) -> impl IntoResponse {
    // Extract signature header value as String before consuming the body
    let signature = request
        .headers()
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Get body bytes for signature verification
    let body_bytes = match request.body_mut().collect().await {
        Ok(collected) => collected.to_bytes().to_vec(),
        Err(e) => {
            error!("Failed to read webhook body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    let Some(signature) = signature else {
        error!("Missing Stripe signature header");
        return (StatusCode::BAD_REQUEST, "Missing stripe-signature header").into_response();
    };

    // Verify the webhook signature
    let stripe_client = match StripeClient::from_env() {
        Ok(client) => client,
        Err(e) => {
            error!("Stripe not configured: {}", e);
            return (StatusCode::SERVICE_UNAVAILABLE, "Stripe not configured").into_response();
        }
    };

    let event = match stripe_client.verify_webhook_signature(&body_bytes, &signature) {
        Ok(event) => event,
        Err(e) => {
            error!("Webhook signature verification failed: {}", e);
            return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
        }
    };

    info!("Received webhook event: {} ({})", event.id, event.event_type);

    // Process the event
    if let Err(e) = process_webhook_event(&event, &state).await {
        error!("Failed to process webhook event {}: {}", event.id, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Processing error: {}", e)).into_response();
    }

    (StatusCode::OK, "OK").into_response()
}

/// Process a verified webhook event
async fn process_webhook_event(event: &WebhookEvent, state: &AppState) -> Result<()> {
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            handle_checkout_completed(event, state).await?;
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(event, state).await?;
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(event, state).await?;
        }
        "invoice.payment_failed" => {
            handle_payment_failed(event, state).await?;
        }
        _ => {
            info!("Unhandled webhook event type: {}", event.event_type);
        }
    }

    Ok(())
}

/// Handle checkout.session.completed event
async fn handle_checkout_completed(event: &WebhookEvent, _state: &AppState) -> Result<()> {
    let data = &event.data.object;

    let session_id = data.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let customer_id = data.get("customer").and_then(|v| v.as_str()).unwrap_or_default();
    let subscription_id = data.get("subscription").and_then(|v| v.as_str()).unwrap_or_default();
    let workspace_id = data
        .get("metadata")
        .and_then(|m| m.get("workspace_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    info!(
        "Checkout completed: session={}, customer={}, subscription={}, workspace={}",
        session_id, customer_id, subscription_id, workspace_id
    );

    // If we have a billing service, update the workspace metadata
    if super::stripe_client::BillingService::is_available() {
        if let Ok(_billing) = super::stripe_client::BillingService::new() {
            // Update workspace billing metadata with subscription ID
            info!("Would update workspace {} with subscription {}", workspace_id, subscription_id);
        }
    }

    Ok(())
}

/// Handle customer.subscription.updated event
async fn handle_subscription_updated(event: &WebhookEvent, _state: &AppState) -> Result<()> {
    let data = &event.data.object;

    let subscription_id = data.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let status = data.get("status").and_then(|v| v.as_str()).unwrap_or_default();
    let current_period_end = data.get("current_period_end").and_then(|v| v.as_i64()).unwrap_or_default();

    info!(
        "Subscription updated: id={}, status={}, period_end={}",
        subscription_id, status, current_period_end
    );

    // Map Stripe status to our internal status
    let is_active = status == "active" || status == "trialing";

    // If subscription became active, could trigger welcome email, etc.
    if is_active {
        info!("Subscription {} is now active", subscription_id);
    }

    Ok(())
}

/// Handle customer.subscription.deleted event
async fn handle_subscription_deleted(event: &WebhookEvent, _state: &AppState) -> Result<()> {
    let data = &event.data.object;

    let subscription_id = data.get("id").and_then(|v| v.as_str()).unwrap_or_default();

    info!("Subscription deleted: id={}", subscription_id);

    // Clear subscription from workspace - downgrade workspace to free tier
    info!("Would downgrade workspace for subscription {}", subscription_id);

    Ok(())
}

/// Handle invoice.payment_failed event
async fn handle_payment_failed(event: &WebhookEvent, _state: &AppState) -> Result<()> {
    let data = &event.data.object;

    let invoice_id = data.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let customer_id = data.get("customer").and_then(|v| v.as_str()).unwrap_or_default();
    let amount_due = data.get("amount_due").and_then(|v| v.as_u64()).unwrap_or_default();

    warn!(
        "Payment failed: invoice={}, customer={}, amount_due={}",
        invoice_id, customer_id, amount_due
    );

    // Could send dunning email to customer
    // Could flag workspace for limited access

    Ok(())
}

/// Webhook event types enum for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEventType {
    CheckoutCompleted,
    SubscriptionUpdated,
    SubscriptionDeleted,
    PaymentFailed,
    Unknown,
}

impl WebhookEventType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "checkout.session.completed" => Self::CheckoutCompleted,
            "customer.subscription.updated" => Self::SubscriptionUpdated,
            "customer.subscription.deleted" => Self::SubscriptionDeleted,
            "invoice.payment_failed" => Self::PaymentFailed,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_type_parsing() {
        assert_eq!(
            WebhookEventType::from_str("checkout.session.completed"),
            WebhookEventType::CheckoutCompleted
        );
        assert_eq!(
            WebhookEventType::from_str("customer.subscription.updated"),
            WebhookEventType::SubscriptionUpdated
        );
        assert_eq!(
            WebhookEventType::from_str("customer.subscription.deleted"),
            WebhookEventType::SubscriptionDeleted
        );
        assert_eq!(
            WebhookEventType::from_str("invoice.payment_failed"),
            WebhookEventType::PaymentFailed
        );
        assert_eq!(
            WebhookEventType::from_str("unknown.event"),
            WebhookEventType::Unknown
        );
    }
}
