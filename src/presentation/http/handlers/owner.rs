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
    application::{
        errors::AppError,
        ports::owner_repository::OwnerRepository,
        use_cases::owner::{
            create_owner::{CreateOwnerInput, CreateOwnerUseCase},
            get_owner::GetOwnerUseCase,
            list_owners::ListOwnersUseCase,
            update_owner::UpdateOwnerUseCase,
        },
    },
    domain::{
        owner::UpdateOwnerCommand,
        subscription::{FeatureKey, LimitKey},
    },
    infrastructure::db::owner_repository_sqlx::PgOwnerRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::TenantContext,
        http::{
            dto::owner::{AssignOwnerDto, CreateOwnerDto, ListOwnersParams, UpdateOwnerDto},
            responses::owner::OwnerResponse,
        },
        middleware::subscription::{require_feature, require_limit, ResolvedSubscription},
    },
};

/// List property owners
#[utoipa::path(
    get,
    path = "/api/v1/owners",
    params(ListOwnersParams),
    responses(
        (status = 200, description = "List of owners", body = Vec<OwnerResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn list_owners(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListOwnersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = ListOwnersUseCase::new(repo);

    let items = usecase
        .execute(
            ctx.agency.id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(OwnerResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get owner details
#[utoipa::path(
    get,
    path = "/api/v1/owners/{id}",
    params(("id" = Uuid, Path, description = "Owner UUID")),
    responses(
        (status = 200, description = "Owner details", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn get_owner(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = GetOwnerUseCase::new(repo);

    let owner = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(OwnerResponse::from(owner)))
}

/// Create a new property owner
#[utoipa::path(
    post,
    path = "/api/v1/owners",
    request_body = CreateOwnerDto,
    responses(
        (status = 201, description = "Owner created", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn create_owner(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let current = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "owners")
        .await?;
    require_limit(&sub.entitlements, &LimitKey::MaxOwners, current)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = CreateOwnerUseCase::new(repo);

    let owner = usecase
        .execute(CreateOwnerInput {
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            email: dto.email,
            phone: dto.phone,
            company_name: dto.company_name,
            kra_pin: dto.kra_pin,
            bank_name: dto.bank_name,
            bank_account: dto.bank_account,
            mpesa_number: dto.mpesa_number,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(OwnerResponse::from(owner))))
}

/// Update owner details
#[utoipa::path(
    patch,
    path = "/api/v1/owners/{id}",
    params(("id" = Uuid, Path, description = "Owner UUID")),
    request_body = UpdateOwnerDto,
    responses(
        (status = 200, description = "Owner updated", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn update_owner(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = UpdateOwnerUseCase::new(repo);

    let owner = usecase
        .execute(UpdateOwnerCommand {
            id,
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            company_name: dto.company_name,
            kra_pin: dto.kra_pin,
            bank_name: dto.bank_name,
            bank_account: dto.bank_account,
            mpesa_number: dto.mpesa_number,
        })
        .await?;

    Ok(Json(OwnerResponse::from(owner)))
}

/// Assign an owner to a property
#[utoipa::path(
    post,
    path = "/api/v1/properties/{propertyId}/owners",
    params(("propertyId" = Uuid, Path, description = "Property UUID")),
    request_body = AssignOwnerDto,
    responses(
        (status = 204, description = "Owner assigned"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Property or owner not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn assign_owner_to_property(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(property_id): Path<Uuid>,
    Json(dto): Json<AssignOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    repo.assign_to_property(dto.owner_id, property_id, dto.ownership_percent)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
