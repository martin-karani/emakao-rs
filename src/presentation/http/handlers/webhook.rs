use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::instrument;

use crate::{
    presentation::{
        app_state::AppState,
        http::dto::webhook::MpesaCallback,
    },
};

/// Receive an M-Pesa STK Push callback from Safaricom
///
/// **No authentication required** — Safaricom posts to this URL directly.
/// Always responds 200 so Safaricom does not retry; internal failures are
/// logged and handled asynchronously.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/mpesa",
    request_body = MpesaCallback,
    responses(
        (status = 200, description = "Callback accepted (always)"),
    ),
    tag = "Webhooks"
)]
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

    if let Err(e) = state
        .subscription
        .change_plan
        .execute(agency_id, &plan_slug, Some(&mpesa_ref), None)
        .await
    {
        tracing::error!(%agency_id, error = %e, "change_plan failed after mpesa confirmation");
    }

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

/// Receive an M-Pesa Transaction Status Callback from Safaricom
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/mpesa/transaction-status/{agency_id}",
    params(
        ("agency_id" = Uuid, Path, description = "Agency UUID")
    ),
    request_body = crate::presentation::http::dto::webhook::TransactionStatusCallback,
    responses(
        (status = 200, description = "Callback accepted (always)"),
    ),
    tag = "Webhooks"
)]
#[instrument(skip(state, payload), fields(agency_id = %agency_id))]
pub async fn mpesa_transaction_status_callback(
    State(state): State<AppState>,
    axum::extract::Path(agency_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<crate::presentation::http::dto::webhook::TransactionStatusCallback>,
) -> impl IntoResponse {
    let result = &payload.result;

    tracing::info!(
        result_code = result.result_code,
        desc = %result.result_desc,
        "mpesa transaction status callback received"
    );

    if result.result_code != 0 {
        tracing::warn!(
            result_code = result.result_code,
            desc = %result.result_desc,
            "mpesa transaction status query failed"
        );
        return StatusCode::OK.into_response();
    }

    let params = match &result.result_parameters {
        Some(p) => p,
        None => {
            tracing::error!("mpesa transaction status callback missing ResultParameters");
            return StatusCode::OK.into_response();
        }
    };

    let amount_kes = match params.get("Amount") {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => {
            tracing::error!("mpesa transaction status callback missing Amount");
            return StatusCode::OK.into_response();
        }
    };

    let Some(receipt) = params.get("ReceiptNo").and_then(|v| v.as_str()).map(str::to_owned) else {
        tracing::error!("mpesa transaction status callback missing ReceiptNo");
        return StatusCode::OK.into_response();
    };

    // The claim_id is passed in the Remarks field which Daraja sends back (usually in Occasion or something similar depending on the exact Daraja response. But we can also just use the receipt number).
    // Actually, Daraja Transaction Status API does not always return the Remarks back in ResultParameters.
    // Instead of relying on Remarks, we can just find the pending claim by receipt number and agency_id!
    // But since the webhook handler doesn't have a direct repo lookup by receipt, wait, we have `PaymentRepository`. Does it have `find_by_receipt`? No.
    // However, since `SettlePaymentClaimUseCase` takes `claim_id`, we need to find the `claim_id` first.
    // Or we could update `SettlePaymentClaimUseCase` to take `receipt_number` instead and do the lookup itself, but the usecase currently expects `claim_id`.
    
    // To be safe, let's just query the database for the claim_id directly here.
    let pool = state.infra.tenant_pools.platform_pool(); // The payment_claims table is actually in the tenant pool if we use tenant schema. Wait.
    // In Emakao, `payment_claims` is part of the `agency_id` tenant, or is it in the platform DB?
    // Looking at the migrations: `migrations/agency/0001_initial.sql` creates `payment_claims`.
    // So `payment_claims` is in the agency pool!

    let agency_pool = match state.infra.tenant_pools.for_agency(agency_id).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get agency pool for transaction status webhook");
            return StatusCode::OK.into_response();
        }
    };

    // Find the claim ID by receipt
    let claim_id_res = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM payment_claims WHERE reference_code = $1 AND status = 'pending_review' LIMIT 1"
    )
    .bind(&receipt)
    .fetch_optional(&agency_pool)
    .await;

    let claim_id = match claim_id_res {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(receipt = %receipt, "No pending payment claim found for this receipt");
            return StatusCode::OK.into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "DB error finding claim by receipt");
            return StatusCode::OK.into_response();
        }
    };

    // Now call the SettlePaymentClaimUseCase
    let repo = std::sync::Arc::new(crate::infrastructure::db::payment_repository_sqlx::PgPaymentRepo::from(agency_pool.clone()));
    let ledger_repo = std::sync::Arc::new(crate::infrastructure::db::ledger_repository_sqlx::PgLedgerRepo::from(agency_pool.clone()));
    let resident_repo = std::sync::Arc::new(crate::infrastructure::db::resident_repository_sqlx::PgResidentRepo::from(agency_pool.clone()));

    let usecase = crate::application::use_cases::payment::settle_claim::SettlePaymentClaimUseCase::new(
        repo,
        ledger_repo,
        resident_repo,
        state.customisation().providers.clone(),
    );

    let input = crate::application::use_cases::payment::settle_claim::SettlePaymentClaimInput {
        agency_id,
        claim_id,
        mpesa_receipt: receipt,
        amount_kes,
    };

    if let Err(e) = usecase.execute(input).await {
        tracing::error!(error = %e, "Failed to settle payment claim from webhook");
    }

    StatusCode::OK.into_response()
}
