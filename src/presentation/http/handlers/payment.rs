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
    application::use_cases::payment::{
        approve_claim::ApproveClaimUseCase, list_claims::ListClaimsUseCase,
        submit_claim::SubmitClaimUseCase,
    },
    application::{
        errors::AppError,
        use_cases::payment::{approve_claim::ApproveClaimInput, submit_claim::SubmitClaimInput},
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::payment_repository_sqlx::PgPaymentRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::TenantContext,
        http::{
            dto::payment::{ListClaimsParams, ReviewClaimDto, SubmitClaimDto},
            responses::payment::PaymentClaimResponse,
        },
    },
};

/// List payment claims, optionally filtered by property or status
#[utoipa::path(
    get,
    path = "/api/v1/payments",
    params(ListClaimsParams),
    responses(
        (status = 200, description = "List of payment claims",      body = Vec<PaymentClaimResponse>),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Payments",
    security(("bearer_token" = []))
)]
pub async fn list_claims(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ListClaimsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgPaymentRepo::from(ctx.pool));
    let usecase = ListClaimsUseCase::new(repo);

    let items = usecase
        .execute(
            ctx.agency.id,
            params.property_id,
            params.status,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(PaymentClaimResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Submit a new payment claim (resident self-reports)
#[utoipa::path(
    post,
    path = "/api/v1/payments",
    request_body = SubmitClaimDto,
    responses(
        (status = 201, description = "Payment claim submitted",     body = PaymentClaimResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 422, description = "Validation error",            body = ErrorResponse),
    ),
    tag = "Payments",
    security(("bearer_token" = []))
)]
pub async fn submit_claim(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<SubmitClaimDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgPaymentRepo::from(ctx.pool));
    let usecase = SubmitClaimUseCase::new(repo);

    let claim = usecase
        .execute(SubmitClaimInput {
            property_id: dto.property_id,
            agreement_id: dto.agreement_id,
            resident_id: Some(user.user_id),
            submitted_by: user.user_id,
            method_type: dto.method_type,
            amount_kes: dto.amount_kes,
            reference_code: dto.reference_code,
            proof_url: dto.proof_url,
            notes: dto.notes,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(PaymentClaimResponse::from(claim))))
}

/// Approve or reject a pending payment claim
#[utoipa::path(
    post,
    path = "/api/v1/payments/{id}/review",
    params(("id" = Uuid, Path, description = "Payment claim UUID")),
    request_body = ReviewClaimDto,
    responses(
        (status = 200, description = "Claim reviewed",              body = PaymentClaimResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 404, description = "Claim not found",             body = ErrorResponse),
        (status = 422, description = "Validation error",            body = ErrorResponse),
    ),
    tag = "Payments",
    security(("bearer_token" = []))
)]
pub async fn review_claim(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ReviewClaimDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgPaymentRepo::from(ctx.pool));
    let usecase = ApproveClaimUseCase::new(repo);

    let claim = usecase
        .execute(ApproveClaimInput {
            agency_id: ctx.agency.id,
            claim_id: id,
            reviewed_by: user.user_id,
            approve: dto.approve,
            review_notes: dto.review_notes,
            rejection_reason: dto.rejection_reason,
        })
        .await?;

    Ok(Json(PaymentClaimResponse::from(claim)))
}
