// src/presentation/http/handlers/property_billing.rs

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::property_billing_repository::PropertyBillingRepository,
    },
    domain::{auth::AuthenticatedUser, property_billing::PropertyBillingSettings},
    infrastructure::db::property_billing_repository_sqlx::PgPropertyBillingRepo,
    presentation::app_state::AppState,
};

pub async fn get_property_billing_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgPropertyBillingRepo::new(ctx_pool(&state, user.agency_id).await?);
    let settings = repo.get_by_property_id(property_id).await?.ok_or_else(|| {
        AppError::NotFound(format!("billing settings for property {property_id}"))
    })?;

    Ok(Json(settings))
}

#[derive(Debug, serde::Serialize)]
pub struct PropertyBillingSummaryResponse {
    pub property_id: Uuid,
    pub property_name: String,
    pub currency_code: String,
    pub rent_due_day: i32,
    pub late_fee_summary: String,
    pub utility_summary: String,
}

pub async fn get_agency_billing_summary(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgPropertyBillingRepo::new(ctx_pool(&state, user.agency_id).await?);

    let rows = repo.get_agency_summary(user.agency_id).await?;

    let summaries: Vec<PropertyBillingSummaryResponse> = rows
        .into_iter()
        .map(|r| {
            let late_fee = if r.late_fee_value > rust_decimal::Decimal::ZERO {
                format!(
                    "{} {} ({}d grace)",
                    if r.late_fee_type == "flat" {
                        r.currency_code.clone()
                    } else {
                        "".to_string()
                    },
                    r.late_fee_value,
                    r.late_fee_grace_days
                )
            } else {
                "None".to_string()
            };

            let mut utils = Vec::new();
            if r.water_rate_per_unit > rust_decimal::Decimal::ZERO {
                utils.push(format!("Water: {}/unit", r.water_rate_per_unit));
            }
            if r.garbage_fee_kes > rust_decimal::Decimal::ZERO {
                utils.push(format!("Garbage: {}", r.garbage_fee_kes));
            }
            if r.security_fee_kes > rust_decimal::Decimal::ZERO {
                utils.push(format!("Security: {}", r.security_fee_kes));
            }

            PropertyBillingSummaryResponse {
                property_id: r.property_id,
                property_name: r.property_name,
                currency_code: r.currency_code,
                rent_due_day: r.rent_due_day,
                late_fee_summary: late_fee,
                utility_summary: if utils.is_empty() {
                    "None".to_string()
                } else {
                    utils.join(", ")
                },
            }
        })
        .collect();

    Ok(Json(summaries))
}

pub async fn upsert_property_billing_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
    Json(mut settings): Json<PropertyBillingSettings>,
) -> Result<impl IntoResponse, AppError> {
    settings.property_id = property_id;

    let repo = PgPropertyBillingRepo::new(ctx_pool(&state, user.agency_id).await?);
    repo.upsert(settings).await?;

    Ok(StatusCode::OK)
}

async fn ctx_pool(state: &AppState, agency_id: Uuid) -> Result<sqlx::PgPool, AppError> {
    state
        .infra
        .tenant_pools
        .for_agency(agency_id)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))
}
