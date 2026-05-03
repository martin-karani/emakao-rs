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
    application::use_cases::maintenance::{
        create_work_order::CreateWorkOrderUseCase, get_work_order::GetWorkOrderUseCase,
        list_work_orders::ListWorkOrdersUseCase, update_work_order::UpdateWorkOrderUseCase,
    },
    application::{
        errors::AppError,
        use_cases::maintenance::{
            create_work_order::CreateWorkOrderInput, update_work_order::UpdateWorkOrderInput,
        },
    },
    domain::{auth::AuthenticatedUser, subscription::FeatureKey},
    infrastructure::db::maintenance_repository_sqlx::PgMaintenanceRepo,
    presentation::{
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::maintenance::{CreateWorkOrderDto, ListWorkOrdersParams, UpdateWorkOrderDto},
            responses::maintenance::WorkOrderResponse,
        },
        middleware::subscription::{require_feature, ResolvedSubscription},
    },
};

/// List maintenance work orders
#[utoipa::path(
    get,
    path = "/api/v1/work-orders",
    params(ListWorkOrdersParams),
    responses(
        (status = 200, description = "List of work orders", body = Vec<WorkOrderResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Maintenance",
    security(("bearer_token" = []))
)]
pub async fn list_work_orders(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let repo = Arc::new(PgMaintenanceRepo::from(ctx.pool));
    let usecase = ListWorkOrdersUseCase::new(repo);

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
            .map(WorkOrderResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get work order details
#[utoipa::path(
    get,
    path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, description = "Work order details", body = WorkOrderResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
    ),
    tag = "Maintenance",
    security(("bearer_token" = []))
)]
pub async fn get_work_order(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let repo = Arc::new(PgMaintenanceRepo::from(ctx.pool));
    let usecase = GetWorkOrderUseCase::new(repo);

    let order = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(WorkOrderResponse::from(order)))
}

/// Create a new maintenance work order
#[utoipa::path(
    post,
    path = "/api/v1/work-orders",
    request_body = CreateWorkOrderDto,
    responses(
        (status = 201, description = "Work order created", body = WorkOrderResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Maintenance",
    security(("bearer_token" = []))
)]
pub async fn create_work_order(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;
    if dto.vendor_id.is_some() {
        require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;
    }

    let repo = Arc::new(PgMaintenanceRepo::from(ctx.pool));
    let usecase = CreateWorkOrderUseCase::new(repo);

    let order = usecase
        .execute(CreateWorkOrderInput {
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            title: dto.title,
            description: dto.description,
            priority: dto.priority,
            vendor_id: dto.vendor_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkOrderResponse::from(order))))
}

/// Update work order status or vendor
#[utoipa::path(
    patch,
    path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    request_body = UpdateWorkOrderDto,
    responses(
        (status = 200, description = "Work order updated", body = WorkOrderResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Maintenance",
    security(("bearer_token" = []))
)]
pub async fn update_work_order(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;
    if dto.vendor_id.is_some() {
        require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;
    }

    let repo = Arc::new(PgMaintenanceRepo::from(ctx.pool));
    let usecase = UpdateWorkOrderUseCase::new(repo);

    let order = usecase
        .execute(UpdateWorkOrderInput {
            agency_id: ctx.agency.id,
            work_order_id: id,
            status: dto.status,
            vendor_id: dto.vendor_id,
            description: dto.description,
        })
        .await?;

    Ok(Json(WorkOrderResponse::from(order)))
}
