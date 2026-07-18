use axum::{
    extract::{Path, Query},
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
    domain::enums::PortalType,
    infrastructure::db::payment_repository_sqlx::PgPaymentRepo,
    presentation::{
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::payment::{ListClaimsParams, ReviewClaimDto, SubmitClaimDto},
            responses::payment::PaymentClaimResponse,
        },
    },
};

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
    ctx: AgencyContext,
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

/// Submit a new payment reconciliation claim
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
    ctx: AgencyContext,
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
            resident_id: match user.portal {
                PortalType::Resident => Some(user.user_id),
                _ => dto.resident_id,
            },
            unit_id: dto.unit_id,
            submitted_by: user.user_id,
            method_type: dto.method_type,
            amount_kes: dto.amount_kes,
            reference_code: dto.reference_code,
            proof_url: dto.proof_url,
            notes: dto.notes,
            submitted_via: dto.submitted_via.unwrap_or_else(|| match user.portal {
                PortalType::Resident => "tenant_app".to_string(),
                _ => "agent_manual".to_string(),
            }),
            raw_message: dto.raw_message,
            period_label: dto.period_label,
            payment_for: dto.payment_for,
            allocation: dto.allocation.unwrap_or_default(),
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
    ctx: AgencyContext,
    axum::extract::State(state): axum::extract::State<crate::presentation::app_state::AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ReviewClaimDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgPaymentRepo::from(ctx.pool.clone()));
    let ledger_repo = Arc::new(crate::infrastructure::db::ledger_repository_sqlx::PgLedgerRepo::from(ctx.pool.clone()));
    let resident_repo = Arc::new(crate::infrastructure::db::resident_repository_sqlx::PgResidentRepo::from(ctx.pool.clone()));
    
    let usecase = ApproveClaimUseCase::new(
        repo,
        ledger_repo,
        resident_repo,
        state.customisation().providers.clone()
    );

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
