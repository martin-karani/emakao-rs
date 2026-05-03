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
        error::ErrorResponse,
        extractors::TenantContext,
        http::{
            dto::property::{CreatePropertyDto, ListPropertiesParams, UpdatePropertyDto},
            responses::property::PropertyResponse,
        },
        middleware::subscription::{require_limit, ResolvedSubscription},
    },
};

#[utoipa::path(
    get,
    path = "/api/v1/properties",
    params(ListPropertiesParams),
    responses(
        (status = 200, description = "List of properties",           body = Vec<PropertyResponse>),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
        (status = 402, description = "Subscription inactive",        body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
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

/// Get a single property by UUID
#[utoipa::path(
    get,
    path = "/api/v1/properties/{id}",
    params(
        ("id" = Uuid, Path, description = "Property UUID")
    ),
    responses(
        (status = 200, description = "Property found",               body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
        (status = 404, description = "Property not found",           body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
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

/// Create a new property
#[utoipa::path(
    post,
    path = "/api/v1/properties",
    request_body = CreatePropertyDto,
    responses(
        (status = 201, description = "Property created",             body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
        (status = 402, description = "Plan limit reached",           body = ErrorResponse),
        (status = 422, description = "Validation error",             body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
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

/// Update a property's mutable fields
#[utoipa::path(
    put,
    path = "/api/v1/properties/{id}",
    params(
        ("id" = Uuid, Path, description = "Property UUID")
    ),
    request_body = UpdatePropertyDto,
    responses(
        (status = 200, description = "Property updated",             body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
        (status = 404, description = "Property not found",           body = ErrorResponse),
        (status = 422, description = "Validation error",             body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
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

/// Delete a property (hard delete — irreversible)
#[utoipa::path(
    delete,
    path = "/api/v1/properties/{id}",
    params(
        ("id" = Uuid, Path, description = "Property UUID")
    ),
    responses(
        (status = 204, description = "Property deleted"),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
        (status = 404, description = "Property not found",           body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
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
