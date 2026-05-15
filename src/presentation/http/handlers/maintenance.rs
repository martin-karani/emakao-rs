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
        use_cases::maintenance::{
            add_work_order_comment::{AddWorkOrderCommentInput, AddWorkOrderCommentUseCase},
            create_caretaker::{CreateCaretakerInput, CreateCaretakerUseCase},
            create_work_order::{CreateWorkOrderInput, CreateWorkOrderUseCase},
            get_work_order::{
                GetWorkOrderByCodeUseCase, GetWorkOrderUseCase, GetWorkOrdersForCaretakerUseCase,
                GetWorkOrdersForResidentUseCase,
            },
            get_work_order_activity::GetWorkOrderActivityUseCase,
            list_caretakers::ListCaretakersUseCase,
            list_work_order_comments::{
                ListCommentRepliesUseCase, ListWorkOrderCommentsInput, ListWorkOrderCommentsUseCase,
            },
            list_work_orders::{ListWorkOrdersInput, ListWorkOrdersUseCase},
            update_caretaker::{UpdateCaretakerInput, UpdateCaretakerUseCase},
            update_work_order::{UpdateWorkOrderInput, UpdateWorkOrderUseCase},
        },
    },
    domain::{
        auth::AuthenticatedUser,
        enums::{WorkOrderCommentAuthorType, WorkOrderReporterType},
        subscription::FeatureKey,
    },
    infrastructure::db::maintenance_repository_sqlx::PgMaintenanceRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{dto::maintenance::*, responses::maintenance::*},
        middleware::subscription::{require_feature, ResolvedSubscription},
    },
};

// ── Shared repo builder ───────────────────────────────────────────────────────
//
// Every handler builds a repo from the pool and wraps it in an Arc.
// Defined as a macro rather than a function so the pool is moved in the same
// expression without needing an extra clone at each call site.

macro_rules! repo {
    ($ctx:expr) => {
        Arc::new(PgMaintenanceRepo::from($ctx.pool))
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// CARETAKERS
// ─────────────────────────────────────────────────────────────────────────────

/// List caretakers (optionally filtered by property)
#[utoipa::path(
    get, path = "/api/v1/caretakers",
    params(ListCaretakersParams),
    responses(
        (status = 200, body = Vec<CaretakerResponse>),
        (status = 401, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_caretakers(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListCaretakersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = ListCaretakersUseCase::new(repo!(ctx))
        .execute(
            params.property_id,
            params.limit.unwrap_or(50),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(CaretakerResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Create a caretaker for a property
#[utoipa::path(
    post, path = "/api/v1/caretakers",
    request_body = CreateCaretakerDto,
    responses(
        (status = 201, body = CaretakerResponse),
        (status = 401, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_caretaker(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateCaretakerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let ct = CreateCaretakerUseCase::new(repo!(ctx))
        .execute(CreateCaretakerInput {
            property_id: dto.property_id,
            created_by: user.user_id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            email: dto.email,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(CaretakerResponse::from(ct))))
}

/// Update a caretaker
#[utoipa::path(
    patch, path = "/api/v1/caretakers/{id}",
    params(("id" = Uuid, Path, description = "Caretaker UUID")),
    request_body = UpdateCaretakerDto,
    responses(
        (status = 200, body = CaretakerResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn update_caretaker(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateCaretakerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let ct = UpdateCaretakerUseCase::new(repo!(ctx))
        .execute(UpdateCaretakerInput {
            id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            email: dto.email,
            is_active: dto.is_active,
        })
        .await?;

    Ok(Json(CaretakerResponse::from(ct)))
}

// ─────────────────────────────────────────────────────────────────────────────
// WORK ORDERS — staff routes
// ─────────────────────────────────────────────────────────────────────────────

/// List work orders with rich filters
#[utoipa::path(
    get, path = "/api/v1/work-orders",
    params(ListWorkOrdersParams),
    responses(
        (status = 200, body = Vec<WorkOrderResponse>),
        (status = 401, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_work_orders(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = ListWorkOrdersUseCase::new(repo!(ctx))
        .execute(ListWorkOrdersInput {
            agency_id: ctx.agency.id,
            property_id: params.property_id,
            unit_id: params.unit_id,
            status: params.status,
            priority: params.priority,
            category: params.category,
            reporter_type: params.reporter_type,
            assigned_caretaker_id: params.assigned_caretaker_id,
            limit: params.limit.unwrap_or(20),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get a single work order by UUID
#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let order = GetWorkOrderUseCase::new(repo!(ctx))
        .execute(ctx.agency.id, id)
        .await?;

    Ok(Json(WorkOrderResponse::from(order)))
}

/// Get a work order by its human-readable code (e.g. MGRD-0042)
#[utoipa::path(
    get, path = "/api/v1/work-orders/by-code/{code}",
    params(("code" = String, Path, description = "Work order code, e.g. MGRD-0042")),
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order_by_code(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let order = GetWorkOrderByCodeUseCase::new(repo!(ctx))
        .execute(ctx.agency.id, &code)
        .await?;

    Ok(Json(WorkOrderResponse::from(order)))
}

/// Create a work order — staff route (reporter_type defaults to Staff)
#[utoipa::path(
    post, path = "/api/v1/work-orders",
    request_body = CreateWorkOrderDto,
    responses(
        (status = 201, body = WorkOrderResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_work_order(
    State(state): State<AppState>,
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
    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id: ctx.agency.id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: dto.reporter_type.unwrap_or(WorkOrderReporterType::Staff),
            reporter_resident_id: dto.reporter_resident_id,
            reporter_caretaker_id: dto.reporter_caretaker_id,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            vendor_id: dto.vendor_id,
            assigned_to: dto.assigned_to,
            assigned_caretaker_id: dto.assigned_caretaker_id,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            estimated_cost_kes: dto.estimated_cost_kes,
            is_tenant_visible: dto.is_tenant_visible.unwrap_or(true),
            internal_notes: dto.internal_notes,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkOrderResponse::from(order))))
}

/// Update a work order — staff route (full field access)
#[utoipa::path(
    patch, path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    request_body = UpdateWorkOrderDto,
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn update_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;
    if dto.vendor_id.as_ref().and_then(|v| v.as_ref()).is_some() {
        require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;
    }
    let uc = UpdateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(UpdateWorkOrderInput {
            agency_id: ctx.agency.id,
            work_order_id: id,
            actor_id: user.user_id,
            actor_type: "staff".to_string(),
            status: dto.status,
            priority: dto.priority,
            category: dto.category,
            vendor_id: dto.vendor_id,
            assigned_to: dto.assigned_to,
            assigned_caretaker_id: dto.assigned_caretaker_id,
            description: dto.description,
            internal_notes: dto.internal_notes,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            started_at: dto.started_at,
            completed_at: dto.completed_at,
            estimated_cost_kes: dto.estimated_cost_kes,
            actual_cost_kes: dto.actual_cost_kes,
            is_tenant_visible: dto.is_tenant_visible,
            attachments: dto.attachments,
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok(Json(WorkOrderResponse::from(order)))
}

// ─────────────────────────────────────────────────────────────────────────────
// PORTAL ROUTES — caretaker + resident
// ─────────────────────────────────────────────────────────────────────────────

/// Caretaker creates a work order (reporter_type forced to Caretaker)
pub async fn caretaker_create_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id: ctx.agency.id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: WorkOrderReporterType::Caretaker, // forced
            reporter_resident_id: None,
            reporter_caretaker_id: dto.reporter_caretaker_id,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            vendor_id: None, // caretakers cannot assign vendors
            assigned_to: None,
            assigned_caretaker_id: None,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            estimated_cost_kes: None,
            is_tenant_visible: dto.is_tenant_visible.unwrap_or(true),
            internal_notes: None,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkOrderResponse::from(order))))
}

/// List work orders assigned to or reported by the calling caretaker
pub async fn caretaker_list_work_orders(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    // reporter_caretaker_id must be passed as a query param by the portal
    let caretaker_id = params.assigned_caretaker_id.ok_or_else(|| {
        AppError::Validation("assigned_caretaker_id is required on this route".into())
    })?;

    let items = GetWorkOrdersForCaretakerUseCase::new(repo!(ctx))
        .execute(
            caretaker_id,
            params.limit.unwrap_or(20),
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

/// Resident creates a work order from their portal (reporter_type forced to Resident)
pub async fn resident_create_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id: ctx.agency.id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: WorkOrderReporterType::Resident, // forced
            reporter_resident_id: dto.reporter_resident_id,
            reporter_caretaker_id: None,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            // residents cannot set scheduling, cost, or internal data
            vendor_id: None,
            assigned_to: None,
            assigned_caretaker_id: None,
            due_date: None,
            scheduled_at: None,
            estimated_cost_kes: None,
            is_tenant_visible: true,
            internal_notes: None,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    // Residents get the public response — no internal fields exposed
    Ok((
        StatusCode::CREATED,
        Json(WorkOrderPublicResponse::from(order)),
    ))
}

/// List work orders visible to the calling resident
pub async fn resident_list_work_orders(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    // The resident UUID is carried in the query param from the portal
    let resident_id = params.reporter_resident_id.ok_or_else(|| {
        AppError::Validation("reporter_resident_id is required on this route".into())
    })?;

    let uc = GetWorkOrdersForResidentUseCase::new(repo!(ctx));

    let items = uc
        .execute(
            resident_id,
            params.limit.unwrap_or(20),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderPublicResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// COMMENTS
// ─────────────────────────────────────────────────────────────────────────────

/// List comments on a work order (staff — sees internal comments)
#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Work order UUID"),
        ListWorkOrderCommentsParams,
    ),
    responses(
        (status = 200, body = Vec<WorkOrderCommentResponse>),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_work_order_comments(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Query(params): Query<ListWorkOrderCommentsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = ListWorkOrderCommentsUseCase::new(repo!(ctx))
        .execute(ListWorkOrderCommentsInput {
            agency_id: ctx.agency.id,
            work_order_id: id,
            include_internal: true, // staff route — internal comments visible
            top_level_only: params.top_level_only.unwrap_or(false),
            limit: params.limit.unwrap_or(50),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// List comments visible to a resident (internal comments hidden)
pub async fn resident_list_work_order_comments(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Query(params): Query<ListWorkOrderCommentsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = ListWorkOrderCommentsUseCase::new(repo!(ctx))
        .execute(ListWorkOrderCommentsInput {
            agency_id: ctx.agency.id,
            work_order_id: id,
            include_internal: false, // resident route — strip internal comments
            top_level_only: params.top_level_only.unwrap_or(false),
            limit: params.limit.unwrap_or(50),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// List replies for a specific comment
#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/comments/{comment_id}/replies",
    params(
        ("id" = Uuid, Path, description = "Work order UUID"),
        ("comment_id" = Uuid, Path, description = "Parent comment UUID"),
    ),
    responses(
        (status = 200, body = Vec<WorkOrderCommentResponse>),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_comment_replies(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path((work_order_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = ListCommentRepliesUseCase::new(repo!(ctx))
        .execute(ctx.agency.id, work_order_id, comment_id, true, 100, 0)
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Add a comment — staff / caretaker / vendor
#[utoipa::path(
    post, path = "/api/v1/work-orders/{id}/comments",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    request_body = CreateWorkOrderCommentDto,
    responses(
        (status = 201, body = WorkOrderCommentResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_work_order_comment(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(work_order_id): Path<Uuid>,
    Json(dto): Json<CreateWorkOrderCommentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let comment = AddWorkOrderCommentUseCase::new(repo!(ctx))
        .execute(AddWorkOrderCommentInput {
            agency_id: ctx.agency.id,
            work_order_id,
            parent_comment_id: dto.parent_comment_id,
            author_id: user.user_id,
            author_type: dto.author_type.unwrap_or(WorkOrderCommentAuthorType::Staff),
            author_resident_id: dto.author_resident_id,
            author_caretaker_id: dto.author_caretaker_id,
            body: dto.body,
            is_internal: dto.is_internal.unwrap_or(false),
            attachments: dto.attachments.unwrap_or_default(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkOrderCommentResponse::from(comment)),
    ))
}

/// Add a comment — resident portal (is_internal always false, enforced by use case)
pub async fn resident_create_work_order_comment(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(work_order_id): Path<Uuid>,
    Json(dto): Json<CreateWorkOrderCommentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let comment = AddWorkOrderCommentUseCase::new(repo!(ctx))
        .execute(AddWorkOrderCommentInput {
            agency_id: ctx.agency.id,
            work_order_id,
            parent_comment_id: dto.parent_comment_id,
            author_id: user.user_id,
            author_type: WorkOrderCommentAuthorType::Resident, // forced
            author_resident_id: dto.author_resident_id,
            author_caretaker_id: None,
            body: dto.body,
            is_internal: false, // use case also enforces this; belt-and-suspenders
            attachments: dto.attachments.unwrap_or_default(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkOrderCommentResponse::from(comment)),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// ACTIVITY LOG
// ─────────────────────────────────────────────────────────────────────────────

/// Get the activity timeline for a work order (staff only)
#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/activity",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, body = Vec<WorkOrderActivityResponse>),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order_activity(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintWorkOrders)?;

    let items = GetWorkOrderActivityUseCase::new(repo!(ctx))
        .execute(ctx.agency.id, id, 200, 0)
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderActivityResponse::from)
            .collect::<Vec<_>>(),
    ))
}
