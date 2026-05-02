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
    application::use_cases::property::{
        create_property::CreatePropertyUseCase, delete_property::DeletePropertyUseCase,
        get_property::GetPropertyUseCase, list_properties::ListPropertiesUseCase,
        update_property::UpdatePropertyUseCase,
    },
    application::{errors::AppError, use_cases::property::create_property::CreatePropertyInput},
    domain::{auth::AuthenticatedUser, subscription::LimitKey},
    infrastructure::db::property_repository_sqlx::PgPropertyRepo,
    presentation::{
        app_state::AppState,
        extractors::TenantContext,
        http::{
            dto::property::{CreatePropertyDto, ListPropertiesParams, UpdatePropertyDto},
            responses::property::PropertyResponse,
        },
        middleware::subscription::{require_limit, ResolvedSubscription},
    },
};

/// GET /api/v1/properties
pub async fn list_properties(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ListPropertiesParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = ListPropertiesUseCase::new(repo);
    let items = usecase
        .execute(
            ctx.agency.id,
            params.property_type,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(
        items
            .into_iter()
            .map(PropertyResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/properties/:id
pub async fn get_property(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = GetPropertyUseCase::new(repo);
    let property = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(PropertyResponse::from(property)))
}

/// POST /api/v1/properties
pub async fn create_property(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreatePropertyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));
    let usecase = CreatePropertyUseCase::new(repo);

    let current_properties = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "properties")
        .await?;
    require_limit(
        &sub.entitlements,
        &LimitKey::MaxProperties,
        current_properties,
    )?;

    let property = usecase
        .execute(CreatePropertyInput {
            agency_id: ctx.agency.id,
            created_by: user.user_id,
            name: dto.name,
            address: dto.address,
            city: dto.city,
            property_type: dto.property_type,
            config: dto.config,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(PropertyResponse::from(property))))
}

/// PUT /api/v1/properties/:id
pub async fn update_property(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdatePropertyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = UpdatePropertyUseCase::new(repo);

    let property = usecase
        .execute(crate::domain::property::UpdatePropertyCommand {
            id,
            agency_id: ctx.agency.id,
            name: dto.name,
            address: dto.address,
            city: dto.city,
        })
        .await?;

    Ok(Json(PropertyResponse::from(property)))
}

/// DELETE /api/v1/properties/:id
pub async fn delete_property(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = DeletePropertyUseCase::new(repo);
    usecase.execute(ctx.agency.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
