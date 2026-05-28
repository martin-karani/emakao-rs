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
        extractors::AgencyContext,
        http::{
            dto::ledger::{ListLedgerParams, PostChargeDto},
            helpers::permission::check_permission,
            responses::ledger::{BalanceSummaryResponse, LedgerEntryResponse},
        },
    },
};

/// List ledger entries for an agreement
///
/// ISSUE 16 FIX: previously had no permission check — any authenticated staff
/// member could read ledger entries for any agreement UUID they could guess,
/// even across property ownership boundaries within the agency.
/// Now requires `can_view` on `agreement:{id}` via OpenFGA.
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
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn list_entries(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(agreement_id): Path<Uuid>,
    Query(params): Query<ListLedgerParams>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the caller has view access to this specific agreement.
    check_permission(
        &state,
        &ctx,
        &user,
        "can_view",
        &format!("agreement:{agreement_id}"),
    )
    .await?;

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
///
/// ISSUE 16 FIX: same as list_entries — requires `can_view` on the agreement.
#[utoipa::path(
    get,
    path = "/api/v1/agreements/{id}/balance",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    responses(
        (status = 200, description = "Balance summary", body = BalanceSummaryResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Agreement not found", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn get_balance(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(agreement_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "can_view",
        &format!("agreement:{agreement_id}"),
    )
    .await?;

    let repo = Arc::new(PgLedgerRepo::from(ctx.pool));
    let usecase = GetBalanceUseCase::new(repo);

    let summary = usecase.execute(agreement_id).await?;
    Ok(Json(BalanceSummaryResponse::from(summary)))
}

/// Post a manual charge or credit to an agreement ledger
///
/// ISSUE 17 FIX: previously had no permission check at all — any authenticated
/// staff member could post charges or credits against any agreement.
/// Now requires both the custom `ledger:write` permission AND `can_edit` on
/// the agreement via OpenFGA.
#[utoipa::path(
    post,
    path = "/api/v1/agreements/{id}/charges",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    request_body = PostChargeDto,
    responses(
        (status = 201, description = "Entry posted", body = LedgerEntryResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Ledger",
    security(("bearer_token" = []))
)]
pub async fn post_charge(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(agreement_id): Path<Uuid>,
    Json(dto): Json<PostChargeDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    // ── Permission gate (two-layer): custom role + OpenFGA ────────────────
    // 1. Custom role permission — enforced even without an FGA store.
    if let Some(ref custom) = state.custom {
        custom
            .permissions
            .require(user.user_id, ctx.agency.id, "ledger:write")
            .await?;
    }

    // 2. OpenFGA relationship — requires `can_edit` on the agreement.
    check_permission(
        &state,
        &ctx,
        &user,
        "can_edit",
        &format!("agreement:{agreement_id}"),
    )
    .await?;

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
