use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use tracing::instrument;

use crate::presentation::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct MpesaCallback {
    #[serde(rename = "Body")]
    pub body: MpesaCallbackBody,
}

#[derive(Debug, Deserialize)]
pub struct MpesaCallbackBody {
    #[serde(rename = "stkCallback")]
    pub stk_callback: StkCallback,
}

#[derive(Debug, Deserialize)]
pub struct StkCallback {
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResultCode")]
    pub result_code: i32,
    #[serde(rename = "ResultDesc")]
    pub result_desc: String,
    #[serde(rename = "CallbackMetadata")]
    pub callback_metadata: Option<CallbackMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackMetadata {
    #[serde(rename = "Item")]
    pub item: Vec<CallbackItem>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: Option<serde_json::Value>,
}

impl CallbackMetadata {
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.item
            .iter()
            .find(|i| i.name == name)
            .and_then(|i| i.value.as_ref())
    }
}

/// POST /api/v1/webhooks/mpesa
#[instrument(skip(state, payload), fields(checkout_id))]
pub async fn mpesa_callback(
    State(state): State<AppState>,
    Json(payload): Json<MpesaCallback>,
) -> impl IntoResponse {
    let cb = &payload.body.stk_callback;
    tracing::Span::current().record("checkout_id", cb.checkout_request_id.as_str());

    tracing::info!(
        result_code = cb.result_code,
        desc = %cb.result_desc,
        "mpesa callback received"
    );

    // Always respond 200 — Safaricom retries on non-2xx and we must not
    // let internal errors cause duplicate charges.
    if cb.result_code != 0 {
        tracing::warn!(
            result_code = cb.result_code,
            desc = %cb.result_desc,
            "mpesa subscription payment failed or cancelled by user"
        );
        return StatusCode::OK.into_response();
    }

    let meta = match &cb.callback_metadata {
        Some(m) => m,
        None => {
            tracing::error!("mpesa success callback missing CallbackMetadata");
            return StatusCode::OK.into_response();
        }
    };

    // Extract payment details from metadata
    let Some(amount_kes) = meta
        .get("Amount")
        .and_then(|v| v.as_f64())
        .map(|f| f as i32)
    else {
        tracing::error!("mpesa callback missing Amount field");
        return StatusCode::OK.into_response();
    };

    let Some(mpesa_ref) = meta
        .get("MpesaReceiptNumber")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
    else {
        tracing::error!("mpesa callback missing MpesaReceiptNumber field");
        return StatusCode::OK.into_response();
    };

    let mpesa_phone = meta.get("PhoneNumber").and_then(|v| {
        v.as_str()
            .map(str::to_owned)
            .or_else(|| v.as_u64().map(|n| n.to_string()))
    });

    // Atomically settle the pending request — returns None if already settled or expired
    let settled = state
        .subscription
        .repo
        .settle_pending_mpesa_request(&cb.checkout_request_id)
        .await;

    let (agency_id, plan_slug) = match settled {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            tracing::warn!(
                checkout_id = %cb.checkout_request_id,
                "no pending subscription request found (already settled or expired)"
            );
            return StatusCode::OK.into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "db error settling pending mpesa request");
            return StatusCode::OK.into_response();
        }
    };

    tracing::info!(
        %agency_id,
        %plan_slug,
        %mpesa_ref,
        amount_kes,
        "settling subscription payment"
    );

    // Activate / update the subscription for the confirmed plan
    if let Err(e) = state
        .subscription
        .change_plan
        .execute(agency_id, &plan_slug, Some(&mpesa_ref), None)
        .await
    {
        tracing::error!(%agency_id, error = %e, "change_plan failed after mpesa confirmation");
        // Do not return early — still record the invoice so payment isn't lost
    }

    // Record the invoice and reactivate the subscription period
    if let Err(e) = state
        .subscription
        .record_payment
        .execute(agency_id, amount_kes, &mpesa_ref, mpesa_phone.as_deref())
        .await
    {
        tracing::error!(%agency_id, error = %e, "record_payment failed after mpesa confirmation");
    }

    StatusCode::OK.into_response()
}
