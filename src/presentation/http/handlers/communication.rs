use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{ledger_repository::LedgerRepository, resident_repository::ResidentRepository},
    },
    infrastructure::{
        db::resident_repository_sqlx::PgResidentRepo, notifications::dispatcher::Recipient,
    },
    presentation::{app_state::AppState, extractors::AgencyContext},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastNoticeRequest {
    pub subject: String,
    pub body: String,
    /// "sms" | "email" | "both"
    pub channels: String,
    /// Mandatory: filter by property
    pub property_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastStatementRequest {
    pub property_id: Uuid,
}

/// POST /api/v1/communication/broadcast
#[utoipa::path(
    post, path = "/api/v1/communication/broadcast",
    request_body = BroadcastNoticeRequest,
    responses(
        (status = 202, description = "Broadcast started"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Communication", security(("bearer_token" = []))
)]
pub async fn broadcast_notice(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Json(body): Json<BroadcastNoticeRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch active residents for this property
    let resident_repo = PgResidentRepo::new(ctx.pool.clone());

    let residents = resident_repo
        .find_by_property_id(ctx.agency.id, body.property_id)
        .await?;

    let settings = state
        .customisation()
        .settings
        .get_or_load(ctx.agency.id, &ctx.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    // 2. Dispatch to each
    for resident in residents {
        let recipient = Recipient {
            phone: resident.phone.clone(),
            email: resident.email.clone(),
            whatsapp: None,
        };

        let context = serde_json::json!({
            "tenant_name": format!("{} {}", resident.first_name, resident.last_name),
            "notice_subject": body.subject,
            "notice_body": body.body,
        });

        // We use a generic event key for manual notices
        let _ = state
            .customisation()
            .notifications
            .dispatch(
                ctx.agency.id,
                Some(body.property_id),
                "manual.notice",
                recipient,
                context,
                &settings.communication,
            )
            .await;
    }

    Ok(StatusCode::ACCEPTED)
}

/// POST /api/v1/communication/statements
#[utoipa::path(
    post, path = "/api/v1/communication/statements",
    request_body = BroadcastStatementRequest,
    responses(
        (status = 202, description = "Statement broadcast started"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Communication", security(("bearer_token" = []))
)]
pub async fn broadcast_statements(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Json(body): Json<BroadcastStatementRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch all active agreements for this property with resident info
    let agreements_rows = sqlx::query(
        r#"
        SELECT 
            a.id as agreement_id,
            u.unit_number,
            r.first_name,
            r.last_name,
            r.phone
        FROM agreements a
        JOIN units u ON a.unit_id = u.id
        JOIN residents r ON a.resident_id = r.id
        WHERE a.property_id = $1 AND a.status = 'active'
        "#,
    )
    .bind(body.property_id)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| AppError::InternalServer(e.to_string()))?;

    let now = time::OffsetDateTime::now_utc();
    let month_desc = time::format_description::parse("[month repr:long]")
        .map_err(|e| AppError::InternalServer(e.to_string()))?;
    let month_name = now
        .format(&month_desc)
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    let prev_month_name = (now - time::Duration::days(30))
        .format(&month_desc)
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    let year = now.year();
    let today_desc = time::format_description::parse("[day] [month repr:short] [year]")
        .map_err(|e| AppError::InternalServer(e.to_string()))?;
    let today_str = now
        .format(&today_desc)
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    // 2. Process each agreement
    for row in agreements_rows {
        let ag_id: Uuid = row.get("agreement_id");
        let ag_unit: String = row.get("unit_number");
        let ag_phone: String = row.get("phone");

        // Fetch latest invoice for this agreement
        let invoice_row = sqlx::query(
            r#"
            SELECT total_kes, line_items
            FROM invoices
            WHERE agreement_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(ag_id)
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        if let Some(inv_row) = invoice_row {
            let inv_total: rust_decimal::Decimal = inv_row.get("total_kes");
            let inv_line_items: serde_json::Value = inv_row.get("line_items");

            // Calculate balance/arrears
            let ledger_repo = crate::infrastructure::db::ledger_repository_sqlx::PgLedgerRepo::new(
                ctx.pool.clone(),
            );
            let balance = ledger_repo.balance_for_agreement(ag_id).await?;

            let total_bal = balance.outstanding;
            let arrears = total_bal - inv_total;

            // Parse line items to get Rent, Water, Garbage
            let mut rent = rust_decimal::Decimal::ZERO;
            let mut water = rust_decimal::Decimal::ZERO;
            let mut garbage = rust_decimal::Decimal::ZERO;

            if let Ok(items) = serde_json::from_value::<Vec<crate::domain::invoice::InvoiceLineItem>>(
                inv_line_items,
            ) {
                for item in items {
                    match item.line_type.as_deref() {
                        Some("rent") => rent += item.total_kes,
                        Some("utility") if item.description.to_lowercase().contains("water") => {
                            water += item.total_kes
                        }
                        Some("utility") if item.description.to_lowercase().contains("garbage") => {
                            garbage += item.total_kes
                        }
                        _ => {}
                    }
                }
            }

            let context = serde_json::json!({
                "acct_no": ag_id.to_string().split('-').next().unwrap_or(""),
                "month": month_name,
                "prev_month": prev_month_name,
                "unit": ag_unit,
                "arrears": arrears,
                "rent": rent,
                "water": water,
                "garbage": garbage,
                "total": total_bal,
                "today": today_str,
                "year": year,
            });

            let recipient = Recipient {
                phone: Some(ag_phone),
                email: None,
                whatsapp: None,
            };

            let settings = state
                .customisation()
                .settings
                .get_or_load(ctx.agency.id, &ctx.pool)
                .await
                .map_err(|e| AppError::InternalServer(e.to_string()))?;

            let _ = state
                .customisation()
                .notifications
                .dispatch(
                    ctx.agency.id,
                    Some(body.property_id),
                    "billing.statement",
                    recipient,
                    context,
                    &settings.communication,
                )
                .await;
        }
    }

    Ok(StatusCode::ACCEPTED)
}
