use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::tax::{
            file_obligation::{FileObligationInput, FileObligationUseCase},
            get_compliance_summary::GetComplianceSummaryUseCase,
            list_obligations::{ListObligationsInput, ListObligationsUseCase},
            mark_paid::{MarkPaidInput, MarkPaidUseCase},
        },
    },
    domain::{auth::AuthenticatedUser, tax::TaxPeriod},
    infrastructure::db::tax_repository_sqlx::PgTaxRepo,
    presentation::{
        app_state::AppState,
        extractors::AgencyContext,
        http::{
            dto::tax::{
                FileTaxObligationDto, MarkTaxPaidDto, TaxObligationFilterParams, VerifyKraPinDto,
            },
            responses::tax::{TaxComplianceSummaryResponse, TaxObligationResponse},
        },
    },
};

/// Get tax compliance summary for a period
pub async fn get_compliance_summary(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(_user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgTaxRepo::new(ctx.pool.clone(), ctx.agency.id));
    let usecase = GetComplianceSummaryUseCase::new(repo);

    // Default to current month
    let now = time::OffsetDateTime::now_utc().date();
    let period = TaxPeriod::new(now.year(), now.month() as u8);

    let summary = usecase.execute(ctx.agency.id, period).await?;
    Ok(Json(TaxComplianceSummaryResponse::from(summary)))
}

/// List tax obligations
pub async fn list_obligations(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(_user): Extension<AuthenticatedUser>,
    Query(params): Query<TaxObligationFilterParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgTaxRepo::new(ctx.pool.clone(), ctx.agency.id));
    let usecase = ListObligationsUseCase::new(repo);

    let obligations = usecase
        .execute(ListObligationsInput {
            agency_id: ctx.agency.id,
            owner_id: params.owner_id,
            property_id: params.property_id,
            obligation_type: params.obligation_type,
            status: params.status,
            tax_period: None,
            limit: params.limit.unwrap_or(20) as i64,
            offset: params.offset.unwrap_or(0) as i64,
        })
        .await?;

    Ok(Json(
        obligations
            .into_iter()
            .map(TaxObligationResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Mark an obligation as filed
pub async fn file_obligation(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<FileTaxObligationDto>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgTaxRepo::new(ctx.pool.clone(), ctx.agency.id));
    let usecase = FileObligationUseCase::new(repo);

    let obligation = usecase
        .execute(FileObligationInput {
            agency_id: ctx.agency.id,
            obligation_id: id,
            kra_ack_number: dto.kra_ack_number,
        })
        .await?;

    Ok(Json(TaxObligationResponse::from(obligation)))
}

/// Mark an obligation as paid
pub async fn mark_paid(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MarkTaxPaidDto>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgTaxRepo::new(ctx.pool.clone(), ctx.agency.id));
    let usecase = MarkPaidUseCase::new(repo);

    let obligation = usecase
        .execute(MarkPaidInput {
            agency_id: ctx.agency.id,
            obligation_id: id,
            kra_prn: dto.kra_prn,
        })
        .await?;

    Ok(Json(TaxObligationResponse::from(obligation)))
}

/// Verify a KRA PIN (Placeholder)
pub async fn verify_kra_pin(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(_user): Extension<AuthenticatedUser>,
    Json(_dto): Json<VerifyKraPinDto>,
) -> Result<impl IntoResponse, AppError> {
    // This would call an external KRA API or use a scraper
    // For now, return a success placeholder
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "PIN verified successfully (placeholder)",
        "agency_id": ctx.agency.id
    })))
}
