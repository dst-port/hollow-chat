use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::badges;
use crate::error::AppError;
use crate::state::AppState;

const SUPPORTER_BADGE: &str = "supporter";

const LAVA_API_BASE: &str = "https://gate.lava.top";

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub url: String,
}

pub async fn create_checkout(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<CheckoutResponse>, AppError> {
    let api_key = state
        .billing
        .lava_api_key
        .as_ref()
        .ok_or(AppError::BillingNotConfigured)?;
    let offer_id = state
        .billing
        .lava_offer_id
        .as_ref()
        .ok_or(AppError::BillingNotConfigured)?;

    let placeholder_email = format!("{}@users.hollowchat.local", session.username);
    let success_url = format!("{}/billing/success", state.billing.app_base_url);
    let failure_url = format!("{}/billing/failure", state.billing.app_base_url);
    let cancel_url = format!("{}/billing/cancel", state.billing.app_base_url);

    let mut body = serde_json::json!({
        "email": placeholder_email,
        "offerId": offer_id.as_ref(),
        "periodicity": "MONTHLY",
        "successful_return_url": success_url,
        "failure_return_url": failure_url,
        "cancel_return_url": cancel_url,
    });

    // Currency is left for the buyer to choose on lava.top's own checkout page unless
    // LAVA_CURRENCY pins the offer to a single one.
    if let Some(currency) = &state.billing.lava_currency {
        body["currency"] = serde_json::Value::String(currency.to_string());
    }

    let response = state
        .http_client
        .post(format!("{LAVA_API_BASE}/api/v3/invoice"))
        .header("X-Api-Key", api_key.as_ref())
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::BillingProvider)?;

    if !response.status().is_success() {
        return Err(AppError::BillingProvider);
    }

    let payload: Value = response.json().await.map_err(|_| AppError::BillingProvider)?;

    let contract_id = payload
        .get("id")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(AppError::BillingProvider)?;
    let url = payload
        .get("paymentUrl")
        .and_then(Value::as_str)
        .ok_or(AppError::BillingProvider)?
        .to_string();

    sqlx::query("INSERT INTO billing_contracts (contract_id, user_id) VALUES ($1, $2)")
        .bind(contract_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(CheckoutResponse { url }))
}

fn tier_for_status(status: &str) -> &'static str {
    match status {
        "completed" | "subscription-active" => "premium",
        _ => "free",
    }
}

async fn resolve_user_id(
    pool: &sqlx::PgPool,
    contract_id: Uuid,
    parent_contract_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    let direct: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM billing_contracts WHERE contract_id = $1")
            .bind(contract_id)
            .fetch_optional(pool)
            .await?;
    if let Some((user_id,)) = direct {
        return Ok(Some(user_id));
    }

    let Some(parent_id) = parent_contract_id else {
        return Ok(None);
    };

    let via_parent: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM billing_contracts WHERE contract_id = $1")
            .bind(parent_id)
            .fetch_optional(pool)
            .await?;

    if let Some((user_id,)) = via_parent {
        sqlx::query(
            "INSERT INTO billing_contracts (contract_id, user_id) VALUES ($1, $2) \
             ON CONFLICT (contract_id) DO NOTHING",
        )
        .bind(contract_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        return Ok(Some(user_id));
    }

    Ok(None)
}

pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<(), AppError> {
    let webhook_secret = state
        .billing
        .lava_webhook_secret
        .as_ref()
        .ok_or(AppError::BillingNotConfigured)?;

    let provided_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if provided_key != webhook_secret.as_ref() {
        return Err(AppError::Unauthorized);
    }

    let Some(status) = payload.get("status").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(contract_id) = payload
        .get("contractId")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(());
    };
    let parent_contract_id = payload
        .get("parentContractId")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok());

    let Some(user_id) = resolve_user_id(&state.pool, contract_id, parent_contract_id).await? else {
        return Ok(());
    };

    let tier = tier_for_status(status);

    sqlx::query("UPDATE users SET tier = $1, subscription_status = $2 WHERE id = $3")
        .bind(tier)
        .bind(status)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    if tier == "premium" {
        badges::award(&state.pool, user_id, SUPPORTER_BADGE).await?;
    } else {
        badges::revoke(&state.pool, user_id, SUPPORTER_BADGE).await?;
    }

    Ok(())
}

/// Boost slots a premium subscription includes - Nitro-style: not a
/// spend-once currency, just "while premium, you can have up to N servers
/// boosted at a time", freely reassignable. Revalidated against live tier
/// on every allocation, so a lapsed subscription can't keep boosts active.
pub const PREMIUM_BOOST_SLOTS: i64 = 2;

pub async fn user_tier(pool: &sqlx::PgPool, user_id: Uuid) -> Result<String, AppError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT tier FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| "free".to_string()))
}

pub async fn is_premium(pool: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    Ok(user_tier(pool, user_id).await? == "premium")
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub tier: String,
    pub subscription_status: Option<String>,
    pub boost_slots_used: i64,
    pub boost_slots_total: i64,
}

pub async fn status(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<StatusResponse>, AppError> {
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT tier, subscription_status FROM users WHERE id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await?;

    let (used,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM server_boosts WHERE user_id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await?;

    let boost_slots_total = if row.0 == "premium" { PREMIUM_BOOST_SLOTS } else { 0 };

    Ok(Json(StatusResponse {
        tier: row.0,
        subscription_status: row.1,
        boost_slots_used: used,
        boost_slots_total,
    }))
}
