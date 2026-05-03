use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::use_cases::ledger::{
        get_balance::GetBalanceUseCase, list_entries::ListLedgerEntriesUseCase,
        post_charge::PostChargeUseCase,
    },
    application::{errors::AppError, use_cases::ledger::post_charge::PostChargeInput},
    domain::auth::AuthenticatedUser,
    infrastructure::db::ledger_repository_sqlx::PgLedgerRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::TenantContext,
        http::{
            dto::ledger::{ListLedgerParams, PostChargeDto},
            responses::ledger::{BalanceSummaryResponse, LedgerEntryResponse},
        },
    },
};

/// List ledger entries for an agreement
#[utoipa::path(
    get,
    path = "/api/v1/agreements/{id}/ledger",
    params(
        ("id" = Uuid, Path, description = "Agreement UUID"),
        ListLedgerParams
    ),
    responses(
        (status = 200, description = "List of ledger entries", body = Vec<LedgerEntryResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn list_entries(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(agreement_id): Path<Uuid>,
    Query(params): Query<ListLedgerParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgLedgerRepo::from(ctx.pool));
    let usecase = ListLedgerEntriesUseCase::new(repo);

    let entries = usecase
        .execute(
            agreement_id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        entries
            .into_iter()
            .map(LedgerEntryResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get current balance summary for an agreement
#[utoipa::path(
    get,
    path = "/api/v1/agreements/{id}/balance",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    responses(
        (status = 200, description = "Balance summary", body = BalanceSummaryResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Agreement not found", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn get_balance(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(agreement_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgLedgerRepo::from(ctx.pool));
    let usecase = GetBalanceUseCase::new(repo);

    let summary = usecase.execute(agreement_id).await?;
    Ok(Json(BalanceSummaryResponse::from(summary)))
}

/// Post a manual charge or credit to an agreement ledger
#[utoipa::path(
    post,
    path = "/api/v1/agreements/{id}/charges",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    request_body = PostChargeDto,
    responses(
        (status = 201, description = "Entry posted", body = LedgerEntryResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn post_charge(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(agreement_id): Path<Uuid>,
    Json(dto): Json<PostChargeDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgLedgerRepo::from(ctx.pool));
    let usecase = PostChargeUseCase::new(repo);

    let entry = usecase
        .execute(PostChargeInput {
            agreement_id: Some(dto.agreement_id.unwrap_or(agreement_id)),
            unit_id: dto.unit_id,
            resident_id: dto.resident_id,
            owner_id: None,
            entry_type: dto.entry_type,
            amount_kes: dto.amount_kes,
            description: dto.description,
            external_ref: dto.external_ref,
            mpesa_receipt: dto.mpesa_receipt,
            period_start: dto.period_start,
            period_end: dto.period_end,
            metadata: dto.metadata.unwrap_or(serde_json::Value::Null),
            posted_by: user.user_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(LedgerEntryResponse::from(entry))))
}
