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
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::agreement::{CreateAgreementDto, ListAgreementsParams},
            helpers::permission::require_permissions,
            responses::agreement::AgreementResponse,
        },
    },
};

/// List lease agreements, optionally filtered by property
#[utoipa::path(
    get,
    path = "/api/v1/agreements",
    params(ListAgreementsParams),
    responses(
        (status = 200, description = "List of agreements",          body = Vec<AgreementResponse>),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Agreements",
    security(("bearer_token" = []))
)]
pub async fn list_agreements(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Query(params): Query<ListAgreementsParams>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(prop_id) = params.property_id {
        crate::presentation::http::helpers::permission::check_permission(
            &state,
            &ctx,
            &user,
            "can_view",
            &format!("property:{}", prop_id),
        )
        .await?;
    }

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

/// Get a single lease agreement by UUID
#[utoipa::path(
    get,
    path = "/api/v1/agreements/{id}",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    responses(
        (status = 200, description = "Agreement found",             body = AgreementResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 404, description = "Agreement not found",         body = ErrorResponse),
    ),
    tag = "Agreements",
    security(("bearer_token" = []))
)]
pub async fn get_agreement(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    crate::presentation::http::helpers::permission::check_permission(
        &state,
        &ctx,
        &user,
        "can_view",
        &format!("agreement:{}", id),
    )
    .await?;

    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = GetAgreementUseCase::new(repo);
    let agreement = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(AgreementResponse::from(agreement)))
}

/// Create a new lease agreement
#[utoipa::path(
    post,
    path = "/api/v1/agreements",
    request_body = CreateAgreementDto,
    responses(
        (status = 201, description = "Agreement created",           body = AgreementResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 422, description = "Validation error",            body = ErrorResponse),
    ),
    tag = "Agreements",
    security(("bearer_token" = []))
)]
pub async fn create_agreement(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateAgreementDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let store_id = ctx
        .agency
        .fga_store_id
        .clone()
        .ok_or_else(|| {
            AppError::InternalServer(
                "Agency has no OpenFGA store configured — contact platform admin".into(),
            )
        })?;

    require_permissions(
        state.openfga.clone(),
        store_id,
        user.user_id,
        vec![
            ("can_edit", format!("property:{}", dto.property_id)),
            ("manager", format!("unit:{}", dto.unit_id)),
        ],
    )
    .await?;

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

/// Terminate a lease agreement early
#[utoipa::path(
    post,
    path = "/api/v1/agreements/{id}/terminate",
    params(("id" = Uuid, Path, description = "Agreement UUID")),
    responses(
        (status = 204, description = "Agreement terminated"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 404, description = "Agreement not found",         body = ErrorResponse),
    ),
    tag = "Agreements",
    security(("bearer_token" = []))
)]
pub async fn terminate_agreement(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgAgreementRepo::from(ctx.pool));
    let usecase = TerminateAgreementUseCase::new(repo);
    usecase.execute(ctx.agency.id, id, user.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
