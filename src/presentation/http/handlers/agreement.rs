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
    application::use_cases::agreement::{
        create_agreement::CreateAgreementUseCase, get_agreement::GetAgreementUseCase,
        list_agreements::ListAgreementsUseCase, terminate_agreement::TerminateAgreementUseCase,
    },
    application::{errors::AppError, use_cases::agreement::create_agreement::CreateAgreementInput},
    domain::auth::AuthenticatedUser,
    infrastructure::db::agreement_repository_sqlx::PgAgreementRepo,
    presentation::{
        app_state::AppState,
        extractors::TenantContext,
        http::{
            dto::agreement::{CreateAgreementDto, ListAgreementsParams},
            responses::agreement::AgreementResponse,
        },
    },
};

pub async fn list_agreements(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ListAgreementsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = ListAgreementsUseCase::new(repo);
    let items = usecase
        .execute(
            ctx.agency.id,
            params.property_id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(
        items
            .into_iter()
            .map(AgreementResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn get_agreement(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = GetAgreementUseCase::new(repo);
    let agreement = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(AgreementResponse::from(agreement)))
}

pub async fn create_agreement(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateAgreementDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = CreateAgreementUseCase::new(repo);
    let agreement = usecase
        .execute(CreateAgreementInput {
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            resident_id: dto.resident_id,
            start_date: dto.start_date,
            end_date: dto.end_date,
            rent_amount_kes: dto.rent_amount_kes,
            deposit_kes: dto.deposit_kes,
            billing_frequency: dto.billing_frequency,
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(AgreementResponse::from(agreement)),
    ))
}

pub async fn terminate_agreement(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = TerminateAgreementUseCase::new(repo);
    usecase.execute(ctx.agency.id, id, user.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
