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
        extractors::TenantContext,
        http::{
            dto::owner::{AssignOwnerDto, CreateOwnerDto, ListOwnersParams, UpdateOwnerDto},
            responses::owner::OwnerResponse,
        },
        middleware::subscription::{require_feature, require_limit, ResolvedSubscription},
    },
};

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
