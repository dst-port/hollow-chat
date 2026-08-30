use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
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

    // Lava validates the domain against a real-looking TLD - a `.local`
    // placeholder gets rejected outright ("Invalid customer email"), so this
    // has to resolve to something that looks real even though nothing ever
    // gets sent there.
    let placeholder_email = format!("{}@users.hollowchat.org", session.username);
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
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!("lava checkout failed: {status} {body}");
        crate::telegram::notify(format!("Lava checkout failed: {status} {body}"));
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

#[derive(Debug, Deserialize)]
pub struct DonationRequest {
    pub amount: f64,
    pub currency: String,
    pub email: String,
}

/// One-off support payment with a buyer-chosen amount. Unauthenticated -
/// this is reachable from the public landing page and isn't tied to a
/// HollowChat account; it just opens a Lava checkout for whatever amount
/// the person typed. Nothing is recorded against a user and the webhook
/// no-ops for it (no billing_contracts row to resolve).
pub async fn create_donation(
    State(state): State<AppState>,
    Json(req): Json<DonationRequest>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let api_key = state
        .billing
        .lava_api_key
        .as_ref()
        .ok_or(AppError::BillingNotConfigured)?;
    let offer_id = state
        .billing
        .lava_donate_offer_id
        .as_ref()
        .ok_or(AppError::BillingNotConfigured)?;

    let currency = match req.currency.trim().to_uppercase().as_str() {
        "USD" => "USD",
        "EUR" => "EUR",
        "RUB" => "RUB",
        _ => return Err(AppError::BadRequest("unsupported currency".into())),
    };

    if !req.amount.is_finite() || req.amount < 1.0 || req.amount > 100_000.0 {
        return Err(AppError::BadRequest("amount out of range".into()));
    }
    let amount = (req.amount * 100.0).round() / 100.0;

    let email = req.email.trim();
    if email.len() < 3 || email.len() > 254 || !email.contains('@') || !email.contains('.') {
        return Err(AppError::BadRequest("enter a valid email".into()));
    }

    let landing = state
        .billing
        .app_base_url
        .as_ref()
        .trim_end_matches("/app")
        .trim_end_matches('/');
    let body = serde_json::json!({
        "email": email,
        "offerId": offer_id.as_ref(),
        "currency": currency,
        "amount": amount,
        "periodicity": "ONE_TIME",
        "successful_return_url": format!("{landing}/support.html?ok=1"),
        "failure_return_url": format!("{landing}/support.html?failed=1"),
        "cancel_return_url": format!("{landing}/support.html"),
    });

    let response = state
        .http_client
        .post(format!("{LAVA_API_BASE}/api/v3/invoice"))
        .header("X-Api-Key", api_key.as_ref())
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::BillingProvider)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        // Lava enforces its own per-currency min/max and returns a 400 with a
        // human-readable "error" field ("Amount=3 not in allowed limits=(5,
        // 10000) for USD"). Surface that to the buyer instead of a generic
        // provider error; anything else is on us.
        if status == reqwest::StatusCode::BAD_REQUEST {
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| "that amount was rejected by the payment provider".to_string());
            return Err(AppError::BadRequest(msg));
        }
        tracing::error!("lava donation invoice failed: {status} {text}");
        crate::telegram::notify(format!("Lava donation invoice failed: {status} {text}"));
        return Err(AppError::BillingProvider);
    }

    let payload: Value = response.json().await.map_err(|_| AppError::BillingProvider)?;
    let url = payload
        .get("paymentUrl")
        .and_then(Value::as_str)
        .ok_or(AppError::BillingProvider)?
        .to_string();

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
